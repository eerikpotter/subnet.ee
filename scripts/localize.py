"""Prerender each public language; keep paired text in attributes for instant switching.

Text-node metadata avoids duplicate content, IDs, and extra layout wrappers. This
parser handles our authored, balanced HTML/SVG templates, not arbitrary user HTML.
"""
import html
from html.parser import HTMLParser
import json
import re
from urllib.parse import urlsplit, urlunsplit, parse_qsl, urlencode

VOID = set("area base br col embed hr img input link meta param source track wbr".split())
ATTRIBUTES = ("aria-label", "alt", "title", "placeholder", "content")


def dictionary(path):
    return json.loads(path.read_text().split("window.SubnetEnglish =", 1)[1].strip().removesuffix(";"))


def translated(source, english):
    if source in english:
        return english[source]
    for pattern, template, key in [
        (r"(\d+) rakendust", "{count} rakendust", "count"),
        (r"(\d+) tööriista · üks loomeruum", "{count} tööriista · üks loomeruum", "count"),
        (r"([A-Za-z]+Craft) käivitub", "{name} käivitub", "name"),
        (r"([A-Za-z]+Craft) tööala", "{name} tööala", "name"),
        (r"Ava ([A-Za-z]+Craft)", "Ava {name}", "name"),
    ]:
        match = re.fullmatch(pattern, source, re.I)
        if match:
            return english.get(template, template).replace("{" + key + "}", match[1])
    return source


def localized_link(value, language):
    url = urlsplit(value)
    if url.scheme or url.netloc or not value.startswith("/"):
        return value
    path = re.sub(r"^/en/", "/", url.path)
    if path not in ("/", "/tooruum/", "/pilveruum/") and not re.fullmatch(r"/apps/[a-z]+/", path):
        return value
    params = [(k, v) for k, v in parse_qsl(url.query, keep_blank_values=True) if k != "lang"]
    if path.startswith("/apps/"):
        if language == "en":
            params.append(("lang", "en"))
    elif language == "en":
        path = "/en" + path
    return urlunsplit(("", "", path, urlencode(params), url.fragment))


class Element:
    def __init__(self, tag, attrs, raw, closed=False):
        self.tag, self.attrs, self.raw, self.closed = tag, dict(attrs), raw, closed
        self.children = []


class Page(HTMLParser):
    def __init__(self, source):
        super().__init__(convert_charrefs=True)
        self.root = Element("", [], "")
        self.stack = [self.root]
        self.feed(source)

    def handle_starttag(self, tag, attrs):
        node = Element(tag, attrs, self.get_starttag_text(), tag in VOID)
        self.stack[-1].children.append(node)
        if not node.closed:
            self.stack.append(node)

    def handle_startendtag(self, tag, attrs):
        self.stack[-1].children.append(Element(tag, attrs, self.get_starttag_text(), True))

    def handle_endtag(self, tag):
        if len(self.stack) > 1 and self.stack[-1].tag == tag:
            self.stack.pop()
        else:
            raise ValueError(f"Unbalanced template closing tag: {tag}")

    def handle_data(self, text):
        children = self.stack[-1].children
        if children and isinstance(children[-1], str):
            children[-1] += text
        else:
            children.append(text)

    def handle_comment(self, text):
        self.stack[-1].children.append(("<!--" + text + "-->",))

    def handle_decl(self, text):
        self.stack[-1].children.append(("<!" + text + ">",))

    def render(self, english, language, path=None, origin=None):
        index = 1 if language == "en" else 0
        def render(node, skip=False):
            if isinstance(node, tuple):
                return node[0]
            changes, texts, attributes = {}, [], {}
            skip = skip or node.tag in ("script", "style", "option") or "data-no-translate" in node.attrs
            for name in ATTRIBUTES:
                source = node.attrs.get(name)
                if source and (target := translated(source, english)) != source:
                    attributes[name] = [source, target]
                    changes[name] = attributes[name][index]
            children = []
            for i, child in enumerate(node.children):
                if not isinstance(child, str):
                    children.append(render(child, skip))
                    continue
                target = child
                if not skip and child.strip():
                    target = child.replace(child.strip(), translated(child.strip(), english), 1)
                if target != child:
                    texts.append([i, child, target])
                children.append((child if index == 0 else target) if node.tag in ("script", "style") else html.escape(child if index == 0 else target, quote=False))
            if texts:
                changes["data-i18n-text"] = json.dumps(texts, ensure_ascii=False, separators=(",", ":"))
            if attributes:
                changes["data-i18n-attrs"] = json.dumps(attributes, ensure_ascii=False, separators=(",", ":"))
            if node.tag == "html":
                changes["lang"] = language
            if node.attrs.get("property") == "og:locale":
                changes["content"] = "en_GB" if language == "en" else "et_EE"
            if node.tag == "a" and node.attrs.get("href"):
                choice = node.attrs.get("data-language")
                changes["href"] = localized_link(node.attrs["href"], choice or language)
                if choice == language:
                    changes["aria-current"] = "true"
            if node.tag == "head" and path is not None:
                et, en = origin + path, origin + "/en" + path
                canonical = en if index else et
                children.append(f'<link rel="canonical" href="{html.escape(canonical, quote=True)}">\n'
                                f'<link rel="alternate" hreflang="et" href="{html.escape(et, quote=True)}">\n'
                                f'<link rel="alternate" hreflang="en" href="{html.escape(en, quote=True)}">\n'
                                f'<meta property="og:url" content="{html.escape(canonical, quote=True)}">\n')
            if not node.tag:
                return "".join(children)
            opening = node.raw
            for name, value in changes.items():
                attribute = f'{name}="{html.escape(value, quote=True)}"'
                pattern = r"(?<![\w-])" + re.escape(name) + r"\s*=\s*(?:\"[^\"]*\"|'[^']*'|[^\s>]+)"
                if re.search(pattern, opening):
                    opening = re.sub(pattern, lambda _: attribute, opening, count=1)
                else:
                    opening = re.sub(r"\s*/?>$", lambda m: " " + attribute + m[0], opening)
            # Keep SVG tag/attribute casing from the original source.
            closing = "" if node.closed else "</" + re.match(r"<([^\s/>]+)", node.raw)[1] + ">"
            return opening + "".join(children) + closing
        return render(self.root)
