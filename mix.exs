defmodule ExOpenDirectory.MixProject do
  use Mix.Project

  @version "0.1.1"
  @source_url "https://github.com/HeroesLament/ex_open_directory"

  def project do
    [
      app: :ex_open_directory,
      version: @version,
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      description: description(),
      package: package(),
      docs: docs(),
      name: "ExOpenDirectory",
      source_url: @source_url
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.36.1", runtime: false},
      {:ex_doc, ">= 0.0.0", only: :dev, runtime: false}
    ]
  end

  defp description do
    "Elixir bindings to macOS OpenDirectory.framework. Query Active Directory and local directory services for users, groups, attributes, and authentication."
  end

  defp package do
    [
      files: ~w(lib native/ex_open_directory/.cargo native/ex_open_directory/src native/ex_open_directory/Cargo.toml native/ex_open_directory/build.rs mix.exs README.md LICENSE),
      licenses: ["MIT"],
      links: %{"GitHub" => @source_url}
    ]
  end

  defp docs do
    [
      main: "ExOpenDirectory",
      source_ref: "v#{@version}",
      source_url: @source_url
    ]
  end
end
