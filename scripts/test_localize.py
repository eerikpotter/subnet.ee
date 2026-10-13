"""Static content is readable without JS, with safe, reversible paired translations."""
import json
from pathlib import Path
import unittest

from localize import Page, dictionary, localized_link

ROOT = Path(__file__).resolve().parents[1]


def elements(node):
    if hasattr(node, "tag"):
        yield node
        for child in node.children:
            yield from elements(child)


def visible(node):
    if isinstance(node, str):
        return node
    if hasattr(node, "tag"):
        return "".join(visible(child) for child in node.children)
    return ""


class StaticLanguagesTest(unittest.TestCase):
    def test_real_pages_have_translated_content_metadata_links_and_identical_structure(self):
        english = dictionary(ROOT / "portal/translations.js")
        for source, path in [("index.html", "/"), ("tooruum/index.html", "/tooruum/")]:
            template = Page((ROOT / "portal" / source).read_text())
            versions = [Page(template.render(english, lang, path, "https://demo.example")) for lang in ("et", "en")]
            et, en = [list(elements(page.root)) for page in versions]
            self.assertEqual([n.tag for n in et], [n.tag for n in en])
            self.assertEqual([n.attrs.get("id") for n in et], [n.attrs.get("id") for n in en])
            for index, nodes in enumerate((et, en)):
                for node in nodes:
                    for position, *pair in json.loads(node.attrs.get("data-i18n-text", "[]")):
                        self.assertEqual(node.children[position], pair[index])
                title = next(n for n in nodes if n.tag == "title")
                self.assertIn("subnet.ee", visible(title))
                canonical = next(n for n in nodes if n.attrs.get("rel") == "canonical")
                self.assertEqual(canonical.attrs['href'], "https://demo.example" + ("/en" if index else "") + path)
                self.assertEqual({n.attrs['hreflang'] for n in nodes if n.attrs.get("rel") == "alternate"}, {"et", "en"})
            self.assertIn("Estonia’s first" if path == "/" else "What will you", visible(versions[1].root))
            links = [n.attrs for n in en if n.tag == "a"]
            self.assertTrue(any(n.get('data-language') == 'et' and n['href'] == path for n in links))
            self.assertTrue(any(n.get('data-language') == 'en' and n['href'] == '/en' + path for n in links))

    def test_entities_svg_and_untrusted_text_are_preserved_without_duplicate_nodes(self):
        source = '<html lang="et"><head><title>A &amp; B</title></head><body><p>A &amp; B<br>Hallo</p><svg viewBox="0 0 1 1"><linearGradient id="g"/><text>Hallo</text></svg><div data-no-translate>Hallo</div></body></html>'
        output = Page(source).render({'A & B': '"<img>" & safe', 'Hallo': 'Hello'}, 'en')
        self.assertIn('viewBox="0 0 1 1"', output)
        self.assertIn('<linearGradient id="g"/>', output)
        self.assertNotIn('<img>', output)
        nodes = list(elements(Page(output).root))
        self.assertEqual(visible(next(n for n in nodes if n.tag == 'title')), '"<img>" & safe')
        self.assertEqual(visible(next(n for n in nodes if n.tag == 'div')), 'Hallo')
        self.assertEqual(visible(next(n for n in nodes if n.tag == 'text')), 'Hello')
        self.assertFalse(any(n.tag in ('span', 'tspan') for n in nodes))

    def test_static_links_preserve_file_references_and_external_destinations(self):
        self.assertEqual(localized_link('/tooruum/?demo=1#failid', 'en'), '/en/tooruum/?demo=1#failid')
        self.assertEqual(localized_link('/apps/photocraft/?cloud_file=a%26b&webgl=#page=3', 'en'), '/apps/photocraft/?cloud_file=a%26b&webgl=&lang=en#page=3')
        for link in ['https://other.example/', '//other.example/', '/apps/photocraft/editor.html', '#tehnoloogia']:
            self.assertEqual(localized_link(link, 'en'), link)


if __name__ == '__main__':
    unittest.main()
