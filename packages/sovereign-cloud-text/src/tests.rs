use super::*;
use std::io::Write;
fn archive(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, text) in parts {
        writer
            .start_file(
                *name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(text.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
#[test]
fn office_extracts_canonical_text_values_and_locations_in_relationship_order() {
    let docx = archive(&[(
        "word/document.xml",
        r#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Tere </w:t></w:r><w:r><w:t>maailm</w:t></w:r></w:p></w:body></w:document>"#,
    )]);
    let result = extract("a.docx", "", None, &docx);
    assert_eq!(result.outcome, Outcome::Ready);
    assert_eq!(result.sections[0].text, "Tere maailm");
    let xlsx = archive(&[
        (
            "xl/workbook.xml",
            r#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Revenue" r:id="s"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="s" Target="worksheets/sheet9.xml"/></Relationships>"#,
        ),
        (
            "xl/sharedStrings.xml",
            "<sst><si><t>Cloud revenue</t></si></sst>",
        ),
        (
            "xl/worksheets/sheet9.xml",
            "<worksheet><sheetData><row r=\"4\"><c r=\"A4\" t=\"s\"><v>0</v></c><c r=\"B4\"><f>2+2</f><v>4</v></c></row></sheetData></worksheet>",
        ),
    ]);
    let result = extract("a.xlsx", "", None, &xlsx);
    assert_eq!(result.outcome, Outcome::Ready);
    assert_eq!(result.sections[0].location, "sheet:Revenue/row:4");
    assert!(result.sections[0].text.contains("A4: Cloud revenue"));
    assert!(result.sections[0].text.contains("B4: 4"));
    assert!(!result.sections[0].text.contains("2+2"));
    let pptx = archive(&[
        (
            "ppt/presentation.xml",
            r#"<p:presentation xmlns:p="p" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId r:id="s2"/><p:sldId r:id="s1"/></p:sldIdLst></p:presentation>"#,
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            r#"<Relationships><Relationship Id="s1" Target="slides/slide1.xml"/><Relationship Id="s2" Target="slides/slide2.xml"/></Relationships>"#,
        ),
        (
            "ppt/slides/slide1.xml",
            "<a><p><r><t>Second</t></r></p></a>",
        ),
        ("ppt/slides/slide2.xml", "<a><p><r><t>First</t></r></p></a>"),
    ]);
    let result = extract("a.pptx", "", None, &pptx);
    assert_eq!(result.sections[0].text, "First");
    assert!(result.sections[0].location.starts_with("slide:1"));
    assert_eq!(result.sections[1].text, "Second");
}
#[test]
fn bounded_pdf_extracts_text_and_page_provenance() {
    use lopdf::{
        Document, Object, Stream,
        content::{Content, Operation},
        dictionary,
    };
    let mut doc = Document::with_version("1.5");
    let pages = doc.new_object_id();
    let font =
        doc.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let resources = doc.add_object(dictionary! {"Font"=>dictionary!{"F1"=>font}});
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 12.into()]),
            Operation::new("Tj", vec![Object::string_literal("Sovereign search works")]),
            Operation::new("ET", vec![]),
        ],
    }
    .encode()
    .unwrap();
    let content = doc.add_object(Stream::new(dictionary! {}, content));
    let page=doc.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"Resources"=>resources,"Contents"=>content,"MediaBox"=>vec![0.into(),0.into(),600.into(),800.into()]});
    doc.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into(),
    );
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    let result = extract("a.pdf", "application/pdf", None, &bytes);
    assert_eq!(result.outcome, Outcome::Ready);
    assert_eq!(result.sections[0].location, "page:1");
    assert_eq!(
        extract("a.pdf", "application/pdf", Some("pdfcraft.pdf"), &bytes).outcome,
        Outcome::Ready
    );
    assert!(result.sections[0].text.contains("Sovereign search works"));
}
#[test]
fn wordcraft_only_extracts_document_body_and_bounds_output() {
    let bytes=br#"{"wordcraftNativeVersion":1,"body":[{"kind":"para","text":"Visible content"},{"kind":"table","rows":[{"cells":[{"blocks":[{"kind":"para","text":"Cell text"}]}]}]}],"embeddedMedia":{"image":"not searchable"},"comments":{"1":"secret annotation"}}"#;
    let result = extract(
        "a.wcraft.json",
        "application/json",
        Some("wordcraft.wcraft"),
        bytes,
    );
    assert_eq!(result.outcome, Outcome::Ready);
    assert_eq!(result.sections.len(), 2);
    assert_eq!(result.sections[1].text, "Cell text");
    let result = extract("large.txt", "text/plain", None, &vec![b'x'; MAX_TEXT + 20]);
    assert_eq!(result.outcome, Outcome::Partial);
    assert!(result.sections.iter().map(|s| s.text.len()).sum::<usize>() <= MAX_TEXT);
    assert_eq!(
        extract("a.txt", "text/plain", None, &[255]).outcome,
        Outcome::Unreadable
    );
    assert_eq!(
        extract("a.txt", "text/plain", None, &vec![b'x'; MAX_INPUT + 1]).outcome,
        Outcome::TooLarge
    );
    assert_eq!(
        extract("a.bin", "", None, b"hello").outcome,
        Outcome::Unsupported
    );
    assert_eq!(
        extract("a.txt", "text/plain", None, b" ").outcome,
        Outcome::Empty
    );
}
#[test]
fn rejects_xml_entities_and_zip_expansion_and_normalizes_keywords() {
    let xml = "<!DOCTYPE p [<!ENTITY a SYSTEM 'file:///secret'>]><p><t>&a;</t></p>";
    assert_eq!(
        extract("a.docx", "", None, &archive(&[("word/document.xml", xml)])).outcome,
        Outcome::Unreadable
    );
    let expanded = "x".repeat(MAX_XML + 1);
    assert_ne!(
        extract(
            "a.docx",
            "",
            None,
            &archive(&[("word/document.xml", &expanded)])
        )
        .outcome,
        Outcome::Ready
    );
    assert_eq!(
        tokens("TÄHT foo-bar １２"),
        vec!["täht", "foo", "bar", "12"]
    );
}

#[test]
fn repeated_spreadsheet_values_and_distinct_terms_stay_bounded() {
    let big = "Repeated ".repeat(40_000);
    let shared = format!("<sst><si><t>{big}</t></si></sst>");
    let row = format!(
        "<worksheet><sheetData><row r=\"1\">{}</row></sheetData></worksheet>",
        "<c r=\"A1\" t=\"s\"><v>0</v></c>".repeat(1000)
    );
    let xlsx = archive(&[
        (
            "xl/workbook.xml",
            r#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Repeated" r:id="s"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="s" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        ("xl/sharedStrings.xml", &shared),
        ("xl/worksheets/sheet1.xml", &row),
    ]);
    let result = extract("a.xlsx", "", None, &xlsx);
    assert_eq!(result.outcome, Outcome::Partial);
    assert!(result.sections.iter().map(|s| s.text.len()).sum::<usize>() <= MAX_TEXT);
    let text = (0..MAX_TERMS + 100)
        .map(|n| format!("term{n}"))
        .collect::<Vec<_>>()
        .join(" ");
    let result = extract("a.txt", "", None, text.as_bytes());
    let (terms, partial) = terms(&result.sections);
    assert_eq!(terms.len(), MAX_TERMS);
    assert!(partial);
}
#[test]
fn pdf_decompression_limit_does_not_publish_a_complete_index() {
    let mut doc =
        lopdf::Document::load_mem(include_bytes!("../tests/fixtures/document.pdf")).unwrap();
    let content = doc
        .objects
        .values_mut()
        .find_map(|o| match o {
            lopdf::Object::Stream(s) => Some(s),
            _ => None,
        })
        .unwrap();
    content.set_plain_content(vec![b' '; MAX_XML + 1]);
    content.compress().unwrap();
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    assert!(bytes.len() < MAX_XML);
    let result = extract("a.pdf", "application/pdf", None, &bytes);
    assert!(!matches!(result.outcome, Outcome::Ready));
    assert!(result.sections.is_empty());
}
