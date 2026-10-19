use super::*;
fn xml(text: &str) -> Result<roxmltree::Document<'_>, ()> {
    roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
            ..Default::default()
        },
    )
    .map_err(|_| ())
}
fn normalize(base: &str, target: &str) -> Option<String> {
    let path = if target.starts_with('/') {
        target.trim_start_matches('/').into()
    } else {
        format!("{base}/{target}")
    };
    let mut parts = Vec::new();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(p),
        }
    }
    Some(parts.join("/"))
}
fn text(node: roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter(|n| {
            n.is_element()
                && n.tag_name().name() == "t"
                && !n.ancestors().any(|a| a.has_tag_name("del"))
        })
        .filter_map(|n| n.text())
        .collect::<Vec<_>>()
        .join("")
}
pub(super) fn extract(bytes: &[u8], ext: &str, out: &mut Output) -> Result<(), ()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| ())?;
    if archive.len() > 4096 {
        return Err(());
    }
    let mut parts = BTreeMap::new();
    let mut total = 0;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|_| ())?;
        let name = file.name().map_err(|_| ())?.into_owned();
        let relevant = match ext {
            "docx" => {
                name == "word/document.xml"
                    || name == "word/footnotes.xml"
                    || name == "word/endnotes.xml"
                    || (name.starts_with("word/header") || name.starts_with("word/footer"))
                        && name.ends_with(".xml")
            }
            "xlsx" => {
                [
                    "xl/sharedStrings.xml",
                    "xl/workbook.xml",
                    "xl/_rels/workbook.xml.rels",
                ]
                .contains(&name.as_str())
                    || name.starts_with("xl/worksheets/") && name.ends_with(".xml")
            }
            "pptx" => {
                ["ppt/presentation.xml", "ppt/_rels/presentation.xml.rels"].contains(&name.as_str())
                    || name.starts_with("ppt/slides/") && name.ends_with(".xml")
            }
            _ => false,
        };
        if !relevant {
            continue;
        }
        if parts.contains_key(&name) {
            return Err(());
        }
        if parts.len() >= 256
            || file.size() > MAX_XML as u64
            || total + file.size() as usize > 8 * 1024 * 1024
        {
            out.partial = true;
            continue;
        }
        let mut data = Vec::new();
        (&mut file)
            .take(MAX_XML as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|_| ())?;
        if data.len() > MAX_XML || data.len() != file.size() as usize {
            return Err(());
        }
        total += data.len();
        parts.insert(name, String::from_utf8(data).map_err(|_| ())?);
    }
    let mut order: Vec<(String, String)> = Vec::new();
    if ext == "docx" {
        if !parts.contains_key("word/document.xml") {
            return Err(());
        }
        order.push(("word/document.xml".into(), "document".into()));
        for name in parts.keys().filter(|p| p.as_str() != "word/document.xml") {
            order.push((name.clone(), name.clone()));
        }
    } else {
        let (base, main, rel, tag) = if ext == "xlsx" {
            (
                "xl",
                "xl/workbook.xml",
                "xl/_rels/workbook.xml.rels",
                "sheet",
            )
        } else {
            (
                "ppt",
                "ppt/presentation.xml",
                "ppt/_rels/presentation.xml.rels",
                "sldId",
            )
        };
        let root = xml(parts.get(main).ok_or(())?)?;
        let relationships = xml(parts.get(rel).ok_or(())?)?;
        let links: BTreeMap<_, _> = relationships
            .descendants()
            .filter(|n| {
                n.has_tag_name("Relationship") && n.attribute("TargetMode") != Some("External")
            })
            .filter_map(|n| Some((n.attribute("Id")?, normalize(base, n.attribute("Target")?)?)))
            .collect();
        for (i, node) in root
            .descendants()
            .filter(|n| n.has_tag_name(tag))
            .enumerate()
        {
            let id = node
                .attribute((
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
                    "id",
                ))
                .ok_or(())?;
            let Some(path) = links.get(id) else {
                out.partial = true;
                continue;
            };
            let label = if ext == "xlsx" {
                format!("sheet:{}", node.attribute("name").unwrap_or(""))
            } else {
                format!("slide:{}", i + 1)
            };
            order.push((path.clone(), label));
        }
    }
    let mut shared = Vec::new();
    let mut shared_bytes = 0;
    if let Some(strings) = parts.get("xl/sharedStrings.xml") {
        let strings = xml(strings)?;
        for node in strings.descendants().filter(|n| n.has_tag_name("si")) {
            let value = text(node);
            shared_bytes += value.len();
            if shared.len() >= 50_000 || shared_bytes > MAX_XML {
                return Err(());
            }
            shared.push(value);
        }
    }
    for (name, label) in order {
        if out.sections.len() >= MAX_SECTIONS || out.bytes >= MAX_TEXT {
            out.partial = true;
            break;
        }
        let Some(part) = parts.get(&name) else {
            out.partial = true;
            continue;
        };
        let doc = xml(part)?;
        if ext == "xlsx" {
            // Keep rows together for useful snippets; references remain tied to stored cells.
            for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
                let mut values = String::new();
                for cell in row.children().filter(|n| n.has_tag_name("c")) {
                    let value = cell
                        .children()
                        .find(|n| n.has_tag_name("v"))
                        .and_then(|n| n.text())
                        .unwrap_or("");
                    let inline;
                    let content = match cell.attribute("t") {
                        Some("s") => shared
                            .get(value.parse::<usize>().map_err(|_| ())?)
                            .ok_or(())?
                            .as_str(),
                        Some("inlineStr") => {
                            inline = text(cell);
                            &inline
                        }
                        _ => value,
                    };
                    if values.len() + content.len() + 64 > MAX_TEXT {
                        out.partial = true;
                        break;
                    }
                    if !content.trim().is_empty() {
                        values.push_str(cell.attribute("r").unwrap_or(""));
                        values.push_str(": ");
                        values.push_str(&content);
                        values.push('\n');
                    }
                }
                out.push(
                    &format!("{label}/row:{}", row.attribute("r").unwrap_or("")),
                    &values,
                );
                if out.sections.len() >= MAX_SECTIONS || out.bytes >= MAX_TEXT {
                    out.partial = true;
                    break;
                }
            }
        } else {
            for (i, p) in doc
                .descendants()
                .filter(|n| n.has_tag_name("p"))
                .enumerate()
            {
                out.push(&format!("{label}/paragraph:{}", i + 1), &text(p));
                if out.sections.len() >= MAX_SECTIONS || out.bytes >= MAX_TEXT {
                    out.partial = true;
                    break;
                }
            }
        }
    }
    Ok(())
}
