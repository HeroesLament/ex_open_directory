defmodule ExOpenDirectory do
  @moduledoc """
  Elixir bindings to macOS OpenDirectory.framework.

  OpenDirectory is Apple's native directory services framework, providing
  access to local accounts, LDAP servers, and Active Directory domains.
  It communicates with the `opendirectoryd` daemon via private IPC —
  there is no alternative to using this framework on macOS.

  ## Nodes

  All operations start by connecting to a node. A node represents a
  directory source:

      # Local directory (users on this Mac)
      {:ok, node} = ExOpenDirectory.connect(:local)

      # Search across all configured directories
      {:ok, node} = ExOpenDirectory.connect(:search)

      # Specific LDAP/AD server
      {:ok, node} = ExOpenDirectory.connect({:server, "ldap://ad.example.com"})

      # Authenticated connection
      {:ok, node} = ExOpenDirectory.connect({:server, "ldap://ad.example.com"},
        username: "admin", password: "secret")

  ## User Lookups

      {:ok, node} = ExOpenDirectory.connect(:search)

      # Find a user by name
      {:ok, record} = ExOpenDirectory.find_user(node, "jsmith")

      # Get specific attributes
      {:ok, attrs} = ExOpenDirectory.get_attributes(record,
        ["dsAttrTypeStandard:RealName", "dsAttrTypeStandard:EMailAddress"])

      # Find all users matching a query
      {:ok, users} = ExOpenDirectory.query_users(node, "smi",
        match: :begins_with, limit: 50)

  ## Group Lookups

      {:ok, groups} = ExOpenDirectory.get_groups(node, "jsmith")

  ## Authentication

      :ok = ExOpenDirectory.authenticate(node, "jsmith", "password123")

  ## Password Operations

      :ok = ExOpenDirectory.change_password(node, "jsmith", "old_pw", "new_pw")

  ## Platform Requirements

  macOS only. The `opendirectoryd` daemon must be running (it always is
  on a standard macOS install). For Active Directory operations, the Mac
  must be bound to an AD domain or the LDAP server must be reachable.
  """

  use Rustler,
    otp_app: :ex_open_directory,
    crate: "ex_open_directory"

  # ── Node / Connection Management ────────────────────────────────────

  @type node_ref :: reference()
  @type record_ref :: reference()
  @type node_type :: :local | :search | {:server, String.t()}

  @doc """
  Connect to an OpenDirectory node.

  ## Node Types

    * `:local` - The local directory (this Mac's users/groups)
    * `:search` - Search across all configured directories
    * `{:server, url}` - A specific LDAP or AD server

  ## Options (for `:server` type)

    * `:username` - DN or username for authenticated bind
    * `:password` - Password for authenticated bind

  Returns `{:ok, node_ref}` on success.
  """
  @spec connect(node_type(), keyword()) :: {:ok, node_ref()} | {:error, String.t()}
  def connect(_node_type, _opts \\ []),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Close a connection to an OpenDirectory node.
  """
  @spec disconnect(node_ref()) :: :ok
  def disconnect(_node),
    do: :erlang.nif_error(:nif_not_loaded)

  # ── User Operations ─────────────────────────────────────────────────

  @doc """
  Find a user record by account name (short name).

  Returns `{:ok, record_ref}` or `{:error, reason}`.
  """
  @spec find_user(node_ref(), String.t()) :: {:ok, record_ref()} | {:error, String.t()}
  def find_user(_node, _username),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Query users matching a pattern.

  ## Options

    * `:match` - Match type: `:exact`, `:begins_with`, `:contains`, `:ends_with`
      (default: `:begins_with`)
    * `:attribute` - Attribute to search (default: `"dsAttrTypeStandard:RecordName"`)
    * `:limit` - Maximum results (default: 100)
    * `:return_attributes` - List of attribute names to return
      (default: standard attributes)

  Returns a list of maps with the requested attributes.
  """
  @spec query_users(node_ref(), String.t(), keyword()) ::
          {:ok, [map()]} | {:error, String.t()}
  def query_users(_node, _query, _opts \\ []),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Get attributes from a record reference.

  Pass a list of attribute type strings like:
    * `"dsAttrTypeStandard:RealName"` - Display name
    * `"dsAttrTypeStandard:EMailAddress"` - Email
    * `"dsAttrTypeStandard:UniqueID"` - UID number
    * `"dsAttrTypeStandard:PrimaryGroupID"` - Primary GID
    * `"dsAttrTypeStandard:NFSHomeDirectory"` - Home directory
    * `"dsAttrTypeStandard:UserShell"` - Login shell

  For AD-bound Macs, AD attributes are also available:
    * `"dsAttrTypeStandard:SMBHomeDrive"` - AD home drive letter
    * `"dsAttrTypeStandard:SMBProfilePath"` - AD roaming profile path
  """
  @spec get_attributes(record_ref(), [String.t()]) :: {:ok, map()} | {:error, String.t()}
  def get_attributes(_record, _attribute_names),
    do: :erlang.nif_error(:nif_not_loaded)

  # ── Group Operations ────────────────────────────────────────────────

  @doc """
  Get the list of group names a user belongs to.

  Uses OpenDirectory's native nested group resolution, which handles
  AD nested groups correctly.
  """
  @spec get_groups(node_ref(), String.t()) :: {:ok, [String.t()]} | {:error, String.t()}
  def get_groups(_node, _username),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Check if a user is a member of a specific group.

  Handles nested group membership (AD and OD).
  """
  @spec member?(node_ref(), String.t(), String.t()) :: boolean()
  def member?(_node, _username, _group),
    do: :erlang.nif_error(:nif_not_loaded)

  # ── Authentication ──────────────────────────────────────────────────

  @doc """
  Authenticate a user against the directory.

  This performs a real authentication check via OpenDirectory, which
  may trigger Kerberos authentication on AD-bound nodes.

  Returns `:ok` on success or `{:error, reason}` on failure.
  """
  @spec authenticate(node_ref(), String.t(), String.t()) :: :ok | {:error, String.t()}
  def authenticate(_node, _username, _password),
    do: :erlang.nif_error(:nif_not_loaded)

  # ── Password Operations ─────────────────────────────────────────────

  @doc """
  Change a user's password.

  Requires knowing the current password. For AD users, this performs
  an AD password change which enforces AD password policy.
  """
  @spec change_password(node_ref(), String.t(), String.t(), String.t()) ::
          :ok | {:error, String.t()}
  def change_password(_node, _username, _old_password, _new_password),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Get password policy information for a user.

  Returns a map that may include:
    * `:expires_at` - DateTime when password expires (nil if never)
    * `:days_until_expiry` - Integer days remaining
    * `:min_length` - Minimum password length
    * `:requires_alpha` - Whether alpha characters are required
    * `:requires_numeric` - Whether numeric characters are required
    * `:history_count` - Number of remembered passwords

  For AD users, this reads the `msDS-UserPasswordExpiryTimeComputed`
  and related attributes.
  """
  @spec password_policy(node_ref(), String.t()) :: {:ok, map()} | {:error, String.t()}
  def password_policy(_node, _username),
    do: :erlang.nif_error(:nif_not_loaded)
end
