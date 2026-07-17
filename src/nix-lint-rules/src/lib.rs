//! Built-in lint rules.
//!
//! This crate provides all lint rules. Each rule is a struct that implements
//! the `Rule` trait and is annotated with `#[lint(...)]` for metadata.

pub mod no_rec;
pub mod no_with_pkgs_lib;
pub mod no_mkdefault;
pub mod no_mkforce;
pub mod no_mkif_true;
pub mod no_any_type;
pub mod no_lookup_path;
pub mod no_nix_env;
pub mod no_optional;
pub mod no_firewall_disable;
pub mod no_builtin_readfile_secrets;
pub mod no_missing_description;
pub mod no_secrets;
pub mod no_defaults;
pub mod one_program_per_part;
pub mod require_flake_parts;
pub mod require_assertions;
pub mod no_cross_namespace_writes;
pub mod no_cross_module_option_reads;
pub mod bool_equals_true;

pub use no_rec::NoRec;
pub use no_with_pkgs_lib::NoWithPkgsLib;
pub use no_mkdefault::NoMkDefault;
pub use no_mkforce::NoMkForce;
pub use no_mkif_true::NoMkIfTrue;
pub use no_any_type::NoAnyType;
pub use no_lookup_path::NoLookupPath;
pub use no_nix_env::NoNixEnv;
pub use no_optional::NoOptional;
pub use no_firewall_disable::NoFirewallDisable;
pub use no_builtin_readfile_secrets::NoBuiltinReadfileSecrets;
pub use no_missing_description::NoMissingDescription;
pub use no_secrets::NoSecrets;
pub use no_defaults::NoDefaults;
pub use one_program_per_part::OneProgramPerPart;
pub use require_flake_parts::RequireFlakeParts;
pub use require_assertions::RequireAssertions;
pub use no_cross_namespace_writes::NoCrossNamespaceWrites;
pub use no_cross_module_option_reads::NoCrossModuleOptionReads;
pub use bool_equals_true::BoolEqualsTrue;
