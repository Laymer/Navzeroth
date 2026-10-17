//! The linked CascLib never decrypts (V2.2d): its built-in key table is not
//! loaded, and an encrypted frame that names a key from that table is reported
//! as encrypted with an unavailable key, even when that key name is in the map.
#[test]
fn encrypted_frames_with_built_in_key_names_are_never_decrypted() {
    assert!(azeroth_local_data::native_decryption_disabled());
}
