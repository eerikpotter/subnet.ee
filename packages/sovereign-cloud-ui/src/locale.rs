//! Only shared cloud controls are localized; native editors keep their own language.
pub fn language() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    if web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok())
        .and_then(|q| q.get("lang"))
        .as_deref()
        == Some("et")
    {
        return "et";
    }
    "en"
}
pub fn tr(source: &str) -> &str {
    translate(source, language())
}
pub fn translate<'a>(source: &'a str, language: &str) -> &'a str {
    if language != "et" {
        return source;
    }
    match source {
        "Untitled" => "Nimetu",
        "Changes will save to your chosen folder" => "Muudatused salvestatakse valitud kausta",
        "Cloud capacity or item limit reached. Free space or contact the workspace owner. Keep a local copy using Export; retry after the limit is resolved." => {
            "Pilveruumi mahu või failide piir on täis. Vabasta ruumi või pöördu tööruumi omaniku poole. Ekspordi kohalik koopia; salvesta uuesti pärast piirangu lahendamist."
        }
        "The cloud file changed. Keep this document open and save a separate copy to compare the versions." => {
            "Pilves olev fail on muutunud. Jäta dokument avatuks ja salvesta eraldi koopia versioonide võrdlemiseks."
        }
        "Your work remains in the editor. Retry this captured save; newer edits will be saved afterwards." => {
            "Töö on redaktoris alles. Proovi salvestust uuesti; uuemad muudatused salvestatakse seejärel."
        }
        "Cloud space" => "Pilveruum",
        "Save" => "Salvesta",
        "Save a copy…" => "Salvesta koopia…",
        "Document name" => "Dokumendi nimi",
        "Saving name…" => "Salvestan nime…",
        "Enter a valid file name" => "Sisesta sobiv faili nimi",
        "The cloud file changed. Reopen it before renaming." => "Faili nimi on vahepeal muutunud. Ava fail uuesti.",
        "Save As…" => "Salvesta nimega…",
        "Open…" => "Ava…",
        "Cancel" => "Tühista",
        "Sign in to open and save files in your folders and shared folders." => {
            "Logi sisse, et avada ja salvestada faile oma kaustades ning jagatud kaustades."
        }
        "Reopen Internet Identity" => "Ava Internet Identity uuesti",
        "Sign in with Internet Identity" => "Logi sisse Internet Identityga",
        "Preparing sign-in…" => "Valmistan sisselogimist ette…",
        "Retry sign-in initialization" => "Proovi sisselogimist uuesti",
        "Your document is safe in the editor" => "Sinu dokument on redaktoris alles",
        "Retry the captured save, or dismiss it and use Save As for a new copy." => {
            "Proovi salvestamist uuesti või lõpeta see katse ja salvesta uue nimega koopia."
        }
        "Retry save" => "Proovi salvestada uuesti",
        "Dismiss attempt" => "Lõpeta katse",
        "Open cloud file?" => "Kas avada pilvefail?",
        "The current document has unsaved changes. Opening this file replaces them." => {
            "Praeguses dokumendis on salvestamata muudatusi. Selle faili avamine asendab need."
        }
        "Discard changes and open" => "Loobu muudatustest ja ava",
        "Complete sign-in in the Internet Identity window…" => "Lõpeta sisselogimine Internet Identity aknas…",
        "Open or create a document first" => "Ava või loo esmalt dokument",
        "This document was opened with another account. Sign back in, or use Save As to save a copy." => {
            "See dokument avati teise kontoga. Logi selle kontoga sisse või salvesta uue nimega koopia."
        }
        "This file is already saved to cloud. No changes to upload." => "Fail on juba pilve salvestatud. Uusi muudatusi pole.",
        "Sign in before saving" => "Enne salvestamist logi sisse",
        "Secure random IDs are unavailable" => "Turvaliste päringutunnuste loomine pole saadaval",
        "Preparing your file…" => "Valmistan faili ette…",
        "Cloud files can be up to 64 MiB" => "Pilvefaili maht võib olla kuni 64 MiB",
        "Account changed. The captured file was not sent." => "Konto muutus. Faili ei saadetud.",
        "Connecting to your cloud…" => "Ühendan pilveruumiga…",
        "Finishing save…" => "Lõpetan salvestamist…",
        "Uploading your file…" => "Laadin faili üles…",
        "Opening your cloud file…" => "Avan pilvefaili…",
        "The open document changed. Choose Save As again." => "Avatud dokument muutus. Vali uuesti „Salvesta nimega”.",
        "The upload was not committed. Retry the captured file." => "Üleslaadimine jäi lõpetamata. Proovi salvestamist uuesti.",
        "Opened read-only. You can edit locally and use Save As to save your own copy." => {
            "Fail avati vaatamiseks. Saad seda kohapeal muuta ja uue nimega oma koopia salvestada."
        }
        "Saved" => "Salvestatud",
        "Could not prepare your file" => "Faili ettevalmistamine ebaõnnestus",
        "Identity initialization" => "Sisselogimise ettevalmistamine",
        "Cloud saved, but the editor could not update its saved state" => "Pilve salvestatud, kuid redaktor ei saanud oma salvestusolekut uuendada",
        "Could not open your file" => "Faili avamine ebaõnnestus",
        "Local recovery needs attention" => "Kohalik taastamine vajab tähelepanu",
        "Local drafts" => "Kohalikud mustandid",
        "Drafts stay in this browser for this account. Restore as a new copy, then choose Save As. Clearing browser storage removes them." => {
            "Mustandid jäävad selle konto jaoks siia brauserisse. Taasta uue koopiana ja vali „Salvesta nimega”. Brauseriandmete kustutamine eemaldab mustandid."
        }
        "Restore copy" => "Taasta koopia",
        "Discard draft" => "Kustuta mustand",
        "Restore draft?" => "Kas taastada mustand?",
        "Restoring replaces the open document. Save or export its unsaved work first. The cloud file will not be overwritten." => {
            "Taastamine asendab avatud dokumendi. Salvesta või ekspordi selle muudatused enne jätkamist. Pilvefaili üle ei kirjutata."
        }
        "Restore as a copy" => "Taasta koopiana",
        "Discard local draft?" => "Kas kustutada kohalik mustand?",
        "This permanently removes this local recovery copy. Cloud files are unchanged." => {
            "See kustutab kohaliku taastamiskoopia lõplikult. Pilvefailid jäävad alles."
        }
        "Local draft storage is unavailable or full" => "Kohalik mustandite salvestusruum pole saadaval või on täis",
        "Local draft request failed" => "Kohaliku mustandi toiming ebaõnnestus",
        "Local draft request interrupted" => "Kohaliku mustandi toiming katkes",
        "Local draft transaction interrupted" => "Kohaliku mustandi salvestamine katkes",
        "Local draft storage unavailable" => "Kohalik mustandite salvestusruum pole saadaval",
        "Invalid draft header" => "Mustandi andmed on vigased",
        "Invalid local draft" => "Kohalik mustand on vigane",
        "Local recovery storage is full. Remove old drafts from the Local drafts panel." => {
            "Kohalik taastamisruum on täis. Eemalda vanu mustandeid kohalike mustandite aknas."
        }
        "Draft belongs to another account" => "Mustand kuulub teisele kontole",
        "Draft is no longer available" => "Mustand pole enam saadaval",
        "Draft changed. Refresh the recovery list." => "Mustand muutus. Värskenda mustandite loendit.",
        "Automatically saved" => "Automaatselt salvestatud",
        "Saving…" => "Salvestan…",
        "Unsaved changes" => "Salvestamata muudatused",
        "Save needs attention" => "Salvestamine vajab tähelepanu",
        "Save As to enable autosave" => "Automaatsalvestuse alustamiseks salvesta pilve",
        "Read-only cloud file" => "Pilvefail on ainult vaatamiseks",
        "Refresh" => "Värskenda",
        _ => source,
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn translated_controls_preserve_dynamic_unknown_content() {
        assert_eq!(super::translate("Saving…", "et"), "Salvestan…");
        assert_eq!(super::translate("Saving…", "en"), "Saving…");
        assert_eq!(super::translate("User file name", "et"), "User file name");
    }
}
