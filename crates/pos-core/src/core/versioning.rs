use pos_types::PosError;

/// Current JSON schema version for command/event envelopes.
pub const SCHEMA_VERSION: u16 = 2;

/// Minimum supported schema version for decoding.
///
/// Default policy: only the current version is supported.
pub const MIN_SUPPORTED_SCHEMA_VERSION: u16 = SCHEMA_VERSION;

/// Maximum supported schema version for decoding.
///
/// Default policy: only the current version is supported.
pub const MAX_SUPPORTED_SCHEMA_VERSION: u16 = SCHEMA_VERSION;

/// Validate a schema version at the boundary (transport/storage).
///
/// Hosts should call this when receiving or loading envelopes to reject
/// unsupported versions before routing or replay.
pub fn ensure_supported_schema(version: u16) -> Result<(), PosError> {
    if version < MIN_SUPPORTED_SCHEMA_VERSION || version > MAX_SUPPORTED_SCHEMA_VERSION {
        return Err(PosError::validation(format!(
            "unsupported schema_version {version}"
        )));
    }

    Ok(())
}
