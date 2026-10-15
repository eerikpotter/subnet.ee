"""Build responsive portal images from Git originals, before localization."""
import hashlib
import html
from html.parser import HTMLParser
import io
import json
from pathlib import Path
import re
import subprocess
import sys


def prepare_images(stage, root):
    # Tiny assembly fixtures and portals without responsive images need no encoder.
    if not any('data-image-widths=' in page.read_text() for page in stage.rglob('*.html')):
        return
    python = root / '.tools/images/bin/python'
    subprocess.run([str(python) if python.exists() else sys.executable,
                    str(Path(__file__).resolve()), str(stage)], check=True)


class ImageTag(HTMLParser):
    def handle_starttag(self, tag, attrs):
        self.attrs = dict(attrs)


def tag(name, attrs):
    return '<' + name + ''.join(
        f' {key}' if value is None else f' {key}="{html.escape(str(value), quote=True)}"'
        for key, value in attrs.items()) + '>'


def optimize(stage):
    from PIL import Image, ImageOps, features
    if not features.check('webp'):
        raise RuntimeError('Image build requires Pillow with WebP support')
    output = stage / 'images'
    output.mkdir(exist_ok=True)
    variants = {}
    manifest = {}

    def encode(source, widths):
        key = (source, widths)
        if key in variants:
            return variants[key]
        path = stage / source.lstrip('/')
        if not source.startswith('/') or not path.resolve().is_relative_to(stage.resolve()):
            raise ValueError(f'Image source must be inside the portal: {source}')
        with Image.open(path) as original:
            image = ImageOps.exif_transpose(original).convert('RGBA' if 'A' in original.getbands() or 'transparency' in original.info else 'RGB')
            candidates = []
            for width in sorted({min(width, image.width) for width in widths}):
                if width <= 0:
                    raise ValueError(f'Invalid image width: {width}')
                height = max(1, round(image.height * width / image.width))
                resized = image.resize((width, height), Image.Resampling.LANCZOS)
                buffer = io.BytesIO()
                resized.save(buffer, format='WEBP', quality=82, method=6,
                             icc_profile=original.info.get('icc_profile', b''))
                data = buffer.getvalue()
                digest = hashlib.sha256(data).hexdigest()[:16]
                name = f'{path.stem}-{width}-{digest}.webp'
                (output / name).write_bytes(data)
                url = '/images/' + name
                candidates.append((url, width))
                manifest[url] = dict(source=source, width=width, height=height,
                                     bytes=len(data), originalBytes=path.stat().st_size)
        variants[key] = candidates
        return candidates

    for page in sorted(stage.rglob('*.html')):
        preloads = []

        def replace(match):
            parser = ImageTag()
            parser.feed(match[0])
            attrs = parser.attrs
            if 'data-image-widths' not in attrs:
                return match[0]
            widths = tuple(int(value) for value in attrs.pop('data-image-widths').split(','))
            candidates = encode(attrs['src'], widths)
            preload = 'data-image-preload' in attrs
            attrs.pop('data-image-preload', None)
            attrs['src'] = candidates[0][0]
            if len(candidates) > 1:
                if not attrs.get('sizes'):
                    raise ValueError(f'Responsive image needs sizes in {page}: {attrs["src"]}')
                attrs['srcset'] = ', '.join(f'{url} {width}w' for url, width in candidates)
            if preload:
                link = dict(rel='preload', **{'as': 'image'}, href=attrs['src'], type='image/webp')
                if 'srcset' in attrs:
                    link.update(imagesrcset=attrs['srcset'], imagesizes=attrs['sizes'])
                if 'fetchpriority' in attrs:
                    link['fetchpriority'] = attrs['fetchpriority']
                preloads.append(tag('link', link))
            return tag('img', attrs)

        source = re.sub(r'<img\b[^>]*>', replace, page.read_text())
        if preloads:
            source = source.replace('</head>', '\n'.join(preloads) + '\n</head>', 1)
        page.write_text(source)

    # This small mark is also used by favicons, SVG <image>, the drive and shells.
    # Resize only the assembled copy, retaining the existing PNG URLs everywhere.
    brand = stage / 'assets/brand/subnet.png'
    if brand.exists():
        with Image.open(brand) as original:
            resized = ImageOps.exif_transpose(original)
            resized.thumbnail((192, 192), Image.Resampling.LANCZOS)
            resized.save(brand, optimize=True)
    (stage / 'image-assets.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(f'Images: {len(manifest)} WebP variants, {sum(item["bytes"] for item in manifest.values()):,} bytes total')


if __name__ == '__main__':
    try:
        optimize(Path(sys.argv[1]))
    except ImportError as error:
        raise SystemExit('Install the image build dependency once:\n'
                         'python3 -m venv .tools/images\n'
                         '.tools/images/bin/python -m pip install -r scripts/image-requirements.txt') from error
