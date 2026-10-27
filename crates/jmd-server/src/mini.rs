//! HTML for the WeChat mini-program's `rich-text` component, which ignores
//! `<style>` blocks: the export HTML body with every rule written inline.

fn tag_style(tag: &str) -> &'static str {
    match tag {
        "h1" => "font-size:24px;font-weight:700;line-height:1.3;margin:0 0 12px;",
        "h2" => "font-size:19px;font-weight:600;line-height:1.35;margin:22px 0 10px;padding-bottom:6px;border-bottom:1px solid #ECEEF2;",
        "h3" => "font-size:16px;font-weight:600;margin:18px 0 8px;",
        "h4" | "h5" | "h6" => "font-size:15px;font-weight:600;margin:14px 0 6px;",
        "p" => "margin:0 0 12px;line-height:1.7;",
        "ul" | "ol" => "margin:0 0 12px;padding-left:22px;line-height:1.7;",
        "li" => "margin:2px 0;",
        "blockquote" => "margin:0 0 12px;padding:8px 12px;background:#F4F7FE;border-left:3px solid #C9D6F5;color:#2A3550;",
        "pre" => "margin:0 0 12px;padding:10px 12px;background:#1E2230;color:#E6E9F0;border-radius:6px;font-size:12px;line-height:1.6;white-space:pre-wrap;word-break:break-all;",
        "code" => "font-family:Menlo,monospace;font-size:13px;background:#F0F2F5;padding:1px 4px;border-radius:3px;",
        "table" => "width:100%;border-collapse:collapse;margin:0 0 12px;font-size:13px;",
        "th" => "border:1px solid #E3E6EB;padding:6px 8px;background:#F4F5F7;text-align:left;",
        "td" => "border:1px solid #E3E6EB;padding:6px 8px;",
        "img" => "max-width:100%;height:auto;",
        "a" => "color:#2F5BD3;text-decoration:none;",
        "hr" => "border:0;border-top:1px solid #E3E6EB;margin:16px 0;",
        _ => "",
    }
}

fn class_style(class: &str) -> String {
    let mut out = String::new();
    for c in class.split_whitespace() {
        out.push_str(match c {
            "toc" => "display:block;background:#FAFBFC;border:1px solid #E3E6EB;border-radius:8px;padding:10px 14px;margin:0 0 16px;",
            "num" => "color:#9AA1AD;margin-right:6px;",
            "task" => "list-style:none;margin-left:-20px;",
            "alert" => "margin:0 0 12px;padding:8px 12px;border-left:3px solid #2F5BD3;background:#F4F7FE;border-radius:4px;",
            "alert-warning" | "alert-caution" => "border-left-color:#B45309;background:#FEF6E7;",
            "alert-tip" => "border-left-color:#1F7A4D;background:#ECF8F1;",
            "alert-title" => "font-weight:600;margin:0 0 4px;",
            "cover" => "display:block;text-align:center;margin:24px 0;",
            "meta" => "color:#5B6270;",
            "footnotes" => "display:block;font-size:13px;color:#5B6270;border-top:1px solid #E3E6EB;margin-top:20px;padding-top:8px;",
            "math" => "font-family:Menlo,monospace;color:#6B3FA0;",
            _ => "",
        });
    }
    out
}

/// Body of `html` with styles inlined and task checkboxes turned into ☑/☐.
pub fn inline_styles(html: &str) -> String {
    let body = html
        .split_once("<body>")
        .map(|(_, b)| b)
        .and_then(|b| b.rsplit_once("</body>").map(|(b, _)| b))
        .unwrap_or(html);
    let body = body
        .replace("<input type=\"checkbox\" disabled checked>", "☑ ")
        .replace("<input type=\"checkbox\" disabled>", "☐ ");
    let mut out = String::with_capacity(body.len() * 2);
    let mut rest = body.as_str();
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        rest = &rest[lt..];
        let Some(gt) = rest.find('>') else { break };
        let tag_src = &rest[..=gt];
        rest = &rest[gt + 1..];
        if tag_src.starts_with("</") || tag_src.starts_with("<!") {
            out.push_str(tag_src);
            continue;
        }
        let inner = tag_src[1..tag_src.len() - 1].trim_end_matches('/');
        let name: String = inner
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        let class = inner
            .split_once("class=\"")
            .and_then(|(_, c)| c.split_once('"'))
            .map(|(c, _)| c)
            .unwrap_or("");
        let mut style = tag_style(&name.to_ascii_lowercase()).to_string();
        style.push_str(&class_style(class));
        if name == "a" && inner.contains("href=\"#") {
            style.push_str("color:inherit;");
        }
        if style.is_empty() || inner.contains("style=") {
            out.push_str(tag_src);
        } else {
            let self_closing = tag_src.ends_with("/>");
            out.push('<');
            out.push_str(inner.trim_end());
            out.push_str(" style=\"");
            out.push_str(&style);
            out.push('"');
            out.push_str(if self_closing { "/>" } else { ">" });
        }
    }
    out.push_str(rest);
    format!(
        "<div style=\"font-size:15px;color:#1B1E24;line-height:1.7;word-wrap:break-word\">{}</div>",
        out.trim()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inlines_styles_and_checkboxes() {
        let html = "<html><head><style>x</style></head><body><h2 id=\"a\"><span class=\"num\">1</span>A</h2><ul><li class=\"task\"><input type=\"checkbox\" disabled checked>Done</li></ul><img src=\"https://x.test/a.png\" alt=\"\"></body></html>";
        let out = inline_styles(html);
        assert!(!out.contains("<style>"));
        assert!(out.contains("<h2 id=\"a\" style=\"font-size:19px"), "{out}");
        assert!(
            out.contains("<span class=\"num\" style=\"color:#9AA1AD"),
            "{out}"
        );
        assert!(out.contains("☑ Done"));
        assert!(
            out.contains("<img src=\"https://x.test/a.png\" alt=\"\" style=\"max-width:100%"),
            "{out}"
        );
    }
}
