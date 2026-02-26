use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AnyThread; // needed for ::alloc()
use objc2_foundation::NSString;
use objc2_open_directory::*;
use rustler::{Atom, Encoder, Env, Error, NifResult, ResourceArc, Term};
use std::sync::Mutex;

mod atoms {
    rustler::atoms! {
        ok,
        error,
        local,
        search,
        server,
        username,
        password,
    }
}

// ── Resource Types ───────────────────────────────────────────────────

struct NodeResource {
    node: Mutex<Retained<ODNode>>,
}

unsafe impl Send for NodeResource {}
unsafe impl Sync for NodeResource {}

struct RecordResource {
    record: Mutex<Retained<ODRecord>>,
}

unsafe impl Send for RecordResource {}
unsafe impl Sync for RecordResource {}

// ── Helpers ──────────────────────────────────────────────────────────

fn ns(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

fn from_ns(s: &NSString) -> String {
    s.to_string()
}

/// Extract error description from an Option<Retained<NSError>> (the objc2 0.3+ pattern).
fn err_to_string(err: &Option<Retained<objc2_foundation::NSError>>) -> String {
    match err {
        Some(e) => from_ns(&e.localizedDescription()),
        None => "Unknown error".to_string(),
    }
}

/// Dereference a kOD* static constant safely. These are `Option<&NSString>` statics
/// in objc2-open-directory 0.3.x. We unwrap and convert to &NSString.
unsafe fn od_const(c: Option<&NSString>) -> &NSString {
    c.expect("OD constant was null")
}

// ── Node / Connection Management ────────────────────────────────────

#[rustler::nif(schedule = "DirtyIo")]
fn connect<'a>(
    env: Env<'a>,
    node_type: Term<'a>,
    opts: Vec<(Atom, Term<'a>)>,
) -> NifResult<(Atom, ResourceArc<NodeResource>)> {
    unsafe {
        let session = ODSession::defaultSession();

        let node: Retained<ODNode> = if let Ok(atom_str) = node_type.atom_to_string() {
            let mut err: Option<Retained<objc2_foundation::NSError>> = None;
            let od_type = match atom_str.as_str() {
                "local" => kODNodeTypeLocalNodes,
                "search" => kODNodeTypeAuthentication,
                _ => {
                    return Err(Error::Term(Box::new(
                        "node_type must be :local, :search, or {:server, url}".to_string(),
                    )))
                }
            };

            match ODNode::initWithSession_type_error(
                ODNode::alloc(),
                session.as_deref(),
                od_type,
                Some(&mut err),
            ) {
                Some(n) => n,
                None => return Err(Error::Term(Box::new(err_to_string(&err)))),
            }
        } else if let Ok((tag, url)) = node_type.decode::<(Atom, String)>() {
            if tag != atoms::server() {
                return Err(Error::Term(Box::new(
                    "node_type must be :local, :search, or {:server, url}".to_string(),
                )));
            }
            let mut err: Option<Retained<objc2_foundation::NSError>> = None;
            match ODNode::initWithSession_name_error(
                ODNode::alloc(),
                session.as_deref(),
                Some(&ns(&url)),
                Some(&mut err),
            ) {
                Some(n) => n,
                None => return Err(Error::Term(Box::new(err_to_string(&err)))),
            }
        } else {
            return Err(Error::Term(Box::new(
                "node_type must be :local, :search, or {:server, url}".to_string(),
            )));
        };

        // Handle authentication opts
        let mut opt_user: Option<String> = None;
        let mut opt_pass: Option<String> = None;
        for (key, val) in &opts {
            if *key == atoms::username() {
                opt_user = Some(
                    val.decode::<String>()
                        .map_err(|_| Error::Term(Box::new("username must be a string".to_string())))?,
                );
            } else if *key == atoms::password() {
                opt_pass = Some(
                    val.decode::<String>()
                        .map_err(|_| Error::Term(Box::new("password must be a string".to_string())))?,
                );
            }
        }

        if let (Some(u), Some(p)) = (opt_user, opt_pass) {
            let mut err: Option<Retained<objc2_foundation::NSError>> = None;
            let ok = node.setCredentialsWithRecordType_recordName_password_error(
                Some(od_const(kODRecordTypeUsers)),
                Some(&ns(&u)),
                Some(&ns(&p)),
                Some(&mut err),
            );
            if !ok {
                return Err(Error::Term(Box::new(err_to_string(&err))));
            }
        }

        Ok((
            atoms::ok(),
            ResourceArc::new(NodeResource {
                node: Mutex::new(node),
            }),
        ))
    }
}

#[rustler::nif]
fn disconnect(_node: ResourceArc<NodeResource>) -> Atom {
    atoms::ok()
}

// ── Internal: find a single user record ──────────────────────────────

unsafe fn find_user_record(
    node: &ODNode,
    username: &str,
) -> Result<Retained<ODRecord>, String> {
    let mut err: Option<Retained<objc2_foundation::NSError>> = None;

    let record_type = od_const(kODRecordTypeUsers);
    let attr_name = od_const(kODAttributeTypeRecordName);
    let standard_only = od_const(kODAttributeTypeStandardOnly);

    let query = ODQuery::initWithNode_forRecordTypes_attribute_matchType_queryValues_returnAttributes_maximumResults_error(
        ODQuery::alloc(),
        Some(node),
        Some(record_type as &AnyObject),
        Some(attr_name),
        kODMatchEqualTo,
        Some(&ns(username) as &AnyObject),
        Some(standard_only as &AnyObject),
        1,
        Some(&mut err),
    ).ok_or_else(|| err_to_string(&err))?;

    let mut err2: Option<Retained<objc2_foundation::NSError>> = None;
    let results = query
        .resultsAllowingPartial_error(false, Some(&mut err2))
        .ok_or_else(|| err_to_string(&err2))?;

    if results.count() == 0 {
        return Err(format!("User '{}' not found", username));
    }

    let obj = results.objectAtIndex(0);
    // The result objects are ODRecord instances behind AnyObject
    let record_ptr = (&*obj) as *const AnyObject as *mut ODRecord;
    Ok(Retained::retain(record_ptr).unwrap())
}

// ── User Operations ──────────────────────────────────────────────────

#[rustler::nif(schedule = "DirtyIo")]
fn find_user(
    node_res: ResourceArc<NodeResource>,
    username: String,
) -> NifResult<(Atom, ResourceArc<RecordResource>)> {
    let node = node_res
        .node
        .lock()
        .map_err(|_| Error::Term(Box::new("lock poisoned".to_string())))?;

    unsafe {
        let record =
            find_user_record(&node, &username).map_err(|e| Error::Term(Box::new(e)))?;

        Ok((
            atoms::ok(),
            ResourceArc::new(RecordResource {
                record: Mutex::new(record),
            }),
        ))
    }
}

#[rustler::nif(schedule = "DirtyIo")]
fn query_users<'a>(
    env: Env<'a>,
    node_res: ResourceArc<NodeResource>,
    query_str: String,
    opts: Vec<(Atom, Term<'a>)>,
) -> NifResult<Term<'a>> {
    let node = node_res
        .node
        .lock()
        .map_err(|_| Error::Term(Box::new("lock poisoned".to_string())))?;

    let mut match_type = kODMatchContains;
    let mut max_results: isize = 100;

    for (key, val) in &opts {
        if *key == Atom::from_str(env, "match").unwrap() {
            if let Ok(mt) = val.atom_to_string() {
                match mt.as_str() {
                    "exact" => match_type = kODMatchEqualTo,
                    "begins_with" => match_type = kODMatchBeginsWith,
                    "contains" => match_type = kODMatchContains,
                    "ends_with" => match_type = kODMatchEndsWith,
                    _ => {}
                }
            }
        } else if *key == Atom::from_str(env, "limit").unwrap() {
            if let Ok(n) = val.decode::<isize>() {
                max_results = n;
            }
        }
    }

    unsafe {
        let record_type = od_const(kODRecordTypeUsers);
        let attr_name = od_const(kODAttributeTypeRecordName);
        let standard_only = od_const(kODAttributeTypeStandardOnly);

        let mut err: Option<Retained<objc2_foundation::NSError>> = None;

        let query = ODQuery::initWithNode_forRecordTypes_attribute_matchType_queryValues_returnAttributes_maximumResults_error(
            ODQuery::alloc(),
            Some(&*node),
            Some(record_type as &AnyObject),
            Some(attr_name),
            match_type,
            Some(&ns(&query_str) as &AnyObject),
            Some(standard_only as &AnyObject),
            max_results,
            Some(&mut err),
        );

        let query = match query {
            Some(q) => q,
            None => return Ok((atoms::ok(), Vec::<String>::new()).encode(env)),
        };

        let mut err2: Option<Retained<objc2_foundation::NSError>> = None;
        let results = match query.resultsAllowingPartial_error(false, Some(&mut err2)) {
            Some(r) => r,
            None => return Ok((atoms::ok(), Vec::<String>::new()).encode(env)),
        };

        let mut names: Vec<String> = Vec::new();
        for i in 0..results.count() {
            let obj = results.objectAtIndex(i);
            let record: &ODRecord = &*((&*obj) as *const AnyObject as *const ODRecord);
            let name = record.recordName();
            names.push(from_ns(&name));
        }

        Ok((atoms::ok(), names).encode(env))
    }
}

#[rustler::nif(schedule = "DirtyIo")]
fn get_attributes<'a>(
    env: Env<'a>,
    record_res: ResourceArc<RecordResource>,
    attribute_names: Vec<String>,
) -> NifResult<Term<'a>> {
    let record = record_res
        .record
        .lock()
        .map_err(|_| Error::Term(Box::new("lock poisoned".to_string())))?;

    unsafe {
        let mut result: Vec<(String, Vec<String>)> = Vec::new();

        for attr_name in &attribute_names {
            let mut err: Option<Retained<objc2_foundation::NSError>> = None;
            let values = record.valuesForAttribute_error(Some(&ns(attr_name)), Some(&mut err));

            let mut strings: Vec<String> = Vec::new();
            if let Some(vals) = values {
                for i in 0..vals.count() {
                    let obj = vals.objectAtIndex(i);
                    let obj_ptr = (&*obj) as *const AnyObject as *mut NSString;
                    if let Some(s) = Retained::retain(obj_ptr) {
                        strings.push(from_ns(&s));
                    }
                }
            }
            result.push((attr_name.clone(), strings));
        }

        Ok((atoms::ok(), result).encode(env))
    }
}

// ── Group Operations ─────────────────────────────────────────────────

fn get_groups_inner(
    node: &ODNode,
    username: &str,
) -> Result<Vec<String>, String> {
    unsafe {
        let record_type = od_const(kODRecordTypeGroups);
        let membership_attr = od_const(kODAttributeTypeGroupMembership);
        let name_attr = od_const(kODAttributeTypeRecordName);

        let mut err: Option<Retained<objc2_foundation::NSError>> = None;
        let query = ODQuery::initWithNode_forRecordTypes_attribute_matchType_queryValues_returnAttributes_maximumResults_error(
            ODQuery::alloc(),
            Some(node),
            Some(record_type as &AnyObject),
            Some(membership_attr),
            kODMatchEqualTo,
            Some(&ns(username) as &AnyObject),
            Some(name_attr as &AnyObject),
            0,
            Some(&mut err),
        );

        let query = match query {
            Some(q) => q,
            None => return Ok(vec![]),
        };

        let mut err2: Option<Retained<objc2_foundation::NSError>> = None;
        let results = match query.resultsAllowingPartial_error(false, Some(&mut err2)) {
            Some(r) => r,
            None => return Ok(vec![]),
        };

        let mut groups: Vec<String> = Vec::new();
        for i in 0..results.count() {
            let obj = results.objectAtIndex(i);
            let record: &ODRecord = &*((&*obj) as *const AnyObject as *const ODRecord);
            let name = record.recordName();
            groups.push(from_ns(&name));
        }

        Ok(groups)
    }
}

#[rustler::nif(schedule = "DirtyIo")]
fn get_groups(
    node_res: ResourceArc<NodeResource>,
    username: String,
) -> NifResult<(Atom, Vec<String>)> {
    let node = node_res
        .node
        .lock()
        .map_err(|_| Error::Term(Box::new("lock poisoned".to_string())))?;

    let groups = get_groups_inner(&node, &username).map_err(|e| Error::Term(Box::new(e)))?;
    Ok((atoms::ok(), groups))
}

#[rustler::nif(name = "member?", schedule = "DirtyIo")]
fn member(
    node_res: ResourceArc<NodeResource>,
    username: String,
    group: String,
) -> NifResult<bool> {
    let node = node_res
        .node
        .lock()
        .map_err(|_| Error::Term(Box::new("lock poisoned".to_string())))?;

    let groups = get_groups_inner(&node, &username).map_err(|e| Error::Term(Box::new(e)))?;
    Ok(groups.contains(&group))
}

// ── Authentication ───────────────────────────────────────────────────

#[rustler::nif(schedule = "DirtyIo")]
fn authenticate(
    node_res: ResourceArc<NodeResource>,
    username: String,
    password: String,
) -> NifResult<Atom> {
    let node = node_res
        .node
        .lock()
        .map_err(|_| Error::Term(Box::new("lock poisoned".to_string())))?;

    unsafe {
        let record =
            find_user_record(&node, &username).map_err(|e| Error::Term(Box::new(e)))?;

        let mut err: Option<Retained<objc2_foundation::NSError>> = None;
        let ok = record.verifyPassword_error(Some(&ns(&password)), Some(&mut err));

        if ok {
            Ok(atoms::ok())
        } else {
            Err(Error::Term(Box::new(err_to_string(&err))))
        }
    }
}

// ── Password Operations ──────────────────────────────────────────────

#[rustler::nif(schedule = "DirtyIo")]
fn change_password(
    node_res: ResourceArc<NodeResource>,
    username: String,
    old_password: String,
    new_password: String,
) -> NifResult<Atom> {
    let node = node_res
        .node
        .lock()
        .map_err(|_| Error::Term(Box::new("lock poisoned".to_string())))?;

    unsafe {
        let record =
            find_user_record(&node, &username).map_err(|e| Error::Term(Box::new(e)))?;

        let mut err: Option<Retained<objc2_foundation::NSError>> = None;
        let ok = record.changePassword_toPassword_error(
            Some(&ns(&old_password)),
            Some(&ns(&new_password)),
            Some(&mut err),
        );

        if ok {
            Ok(atoms::ok())
        } else {
            Err(Error::Term(Box::new(err_to_string(&err))))
        }
    }
}

#[rustler::nif(schedule = "DirtyIo")]
fn password_policy<'a>(
    env: Env<'a>,
    node_res: ResourceArc<NodeResource>,
    username: String,
) -> NifResult<Term<'a>> {
    let node = node_res
        .node
        .lock()
        .map_err(|_| Error::Term(Box::new("lock poisoned".to_string())))?;

    unsafe {
        let record =
            find_user_record(&node, &username).map_err(|e| Error::Term(Box::new(e)))?;

        let mut policy_info: Vec<(String, String)> = Vec::new();

        let policy_attrs = [
            "dsAttrTypeNative:accountPolicyData",
            "dsAttrTypeNative:pwdLastSet",
            "dsAttrTypeNative:msDS-UserPasswordExpiryTimeComputed",
        ];

        for attr in &policy_attrs {
            let mut err: Option<Retained<objc2_foundation::NSError>> = None;
            if let Some(vals) = record.valuesForAttribute_error(Some(&ns(attr)), Some(&mut err)) {
                if vals.count() > 0 {
                    let obj = vals.objectAtIndex(0);
                    let obj_ptr = (&*obj) as *const AnyObject as *mut NSString;
                    if let Some(s) = Retained::retain(obj_ptr) {
                        policy_info.push((attr.to_string(), from_ns(&s)));
                    }
                }
            }
        }

        Ok((atoms::ok(), policy_info).encode(env))
    }
}

// ── NIF Init ─────────────────────────────────────────────────────────

fn load(env: Env, _info: Term) -> bool {
    rustler::resource!(NodeResource, env);
    rustler::resource!(RecordResource, env);
    true
}

rustler::init!("Elixir.ExOpenDirectory", load = load);
