use super::*;
pub(super) fn extract(bytes: &[u8], out: &mut Output) -> Result<(), ()> {
    let doc = lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions::with_max_decompressed_size(MAX_XML),
    )
    .map_err(|_| ())?;
    if doc.is_encrypted() || doc.objects.len() > 50_000 {
        return Err(());
    }
    let pages = doc.get_pages();
    out.partial = pages.len() > 128;
    for page in pages.keys().take(128) {
        if out.sections.len() >= MAX_SECTIONS || out.bytes >= MAX_TEXT {
            out.partial = true;
            break;
        }
        match doc.extract_text_with_limit(&[*page], MAX_XML) {
            Ok(text) => out.push(&format!("page:{page}"), &text),
            Err(_) => out.partial = true,
        }
    }
    Ok(())
}
