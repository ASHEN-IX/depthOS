"""Bake original atmosphere and stage geometry into small static textures."""
from pathlib import Path
from random import Random
import subprocess

OUTPUT = Path(__file__).resolve().parent


def render(name, size, elements):
    width, height = size
    svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">{elements}</svg>'
    args = ["magick", "-background", "none", "svg:-", "-depth", "8", "-strip"]
    if name.endswith(".jpg"):
        args.extend(["-quality", "89"])
    subprocess.run([*args, str(OUTPUT / name)], input=svg, encoding="utf-8", check=True)


def atmosphere():
    elements = ['<rect width="960" height="540" fill="#080d17"/>']
    pools = ((242, 136, 372, "#6629a9"), (702, 90, 344, "#135b70"), (518, 348, 276, "#07463e"))
    for i, (x, y, radius, color) in enumerate(pools):
        elements.append(f'<defs><radialGradient id="g{i}"><stop stop-color="{color}" stop-opacity=".65"/>'
                        f'<stop offset="1" stop-color="{color}" stop-opacity="0"/></radialGradient></defs>'
                        f'<circle cx="{x}" cy="{y}" r="{radius}" fill="url(#g{i})"/>')
    rng = Random(4221)
    for _ in range(1850):
        x, y = rng.randrange(960), rng.randrange(340)
        alpha = rng.uniform(.09, .32) if rng.random() < .10 else rng.uniform(.04, .12)
        elements.append(f'<circle cx="{x}" cy="{y}" r=".57" fill="#94c8ef" opacity="{alpha:.2f}"/>')
    gradient = ('<defs><linearGradient id="arc"><stop stop-color="#8b5cf6" stop-opacity=".35"/>'
                '<stop offset=".22" stop-color="#bb77ff"/><stop offset=".65" stop-color="#54effe"/>'
                '<stop offset="1" stop-color="#626ffa" stop-opacity=".15"/></linearGradient></defs>')
    elements.append(gradient)
    for weight, alpha in ((24, .06), (11, .10), (5, .22), (2, .84)):
        elements.append(f'<path d="M-50 228 Q478 -110 1010 221" fill="none" stroke="url(#arc)" '
                        f'stroke-width="{weight}" opacity="{alpha}"/>')
    for col in range(-13, 14):
        elements.append(f'<path d="M{480 + col * 9} 291 L{480 + col * 113} 540" stroke="#0a5a65" opacity=".37"/>')
    for row in range(15):
        y = 291 + 249 * (row / 14) ** 2.1
        elements.append(f'<path d="M0 {y:.1f} H960" stroke="#0d6267" opacity=".31"/>')
    render("horizon.jpg", (960, 540), "".join(elements))


def orbit():
    elements = ['<defs><linearGradient id="e"><stop stop-color="#22c7f2" stop-opacity=".23"/>'
                '<stop offset=".28" stop-color="#ae7bff"/><stop offset=".52" stop-color="#0d4353"/>'
                '<stop offset=".79" stop-color="#26eaae"/><stop offset="1" stop-color="#22c7f2"/>'
                '</linearGradient></defs>']
    for rx, ry in ((500, 46), (469, 38), (390, 29), (327, 20)):
        for weight, alpha in ((13, .08), (6, .16), (2, .85)):
            elements.append(f'<ellipse cx="512" cy="56" rx="{rx}" ry="{ry}" fill="none" '
                            f'stroke="url(#e)" stroke-width="{weight}" opacity="{alpha}"/>')
    render("orbit.png", (1024, 116), "".join(elements))


def workspace_accents():
    from math import sin

    wave = []
    for band in range(12):
        points = " ".join(f"{x},{65 + 2.2*band + (20 - band*.9)*sin(x/28 + band*.13):.1f}"
                          for x in range(0, 321, 4))
        tone = "#5bdada" if band % 3 else "#b184ff"
        wave.append(f'<polyline points="{points}" fill="none" stroke="{tone}" opacity="{.60-band*.028:.2f}"/>')

    stack = ['<path d="M60 60L171 15L278 49L168 89Z" fill="#265473" stroke="#69ddf1"/>']
    for y in (14, 26, 38):
        stack.insert(0, f'<path d="M60 {60+y}L171 {15+y}L278 {49+y}L168 {89+y}Z" '
                        'fill="#132940" stroke="#2c8aae"/>')
    stack.append('<path d="M105 51l58-23 46 15-57 23Z" fill="#4da0cb" opacity=".7"/>')

    nodes = [(48, 71), (100, 43), (136, 85), (169, 31), (216, 50), (258, 23), (292, 66)]
    network = ['<defs><radialGradient id="node"><stop stop-color="#57e9ee"/>'
               '<stop offset="1" stop-color="#145178"/></radialGradient></defs>']
    network.append('<polyline points="' + " ".join(f"{x},{y}" for x, y in nodes) +
                   '" fill="none" stroke="#4cc0d8" stroke-width="2"/>')
    for index, (x, y) in enumerate(nodes):
        network.append(f'<circle cx="{x}" cy="{y}" r="{9+index%3*3}" fill="url(#node)"/>')

    arcs = []
    for r in range(22, 75, 8):
        arcs.append(f'<path d="M{174-r} 101a{r} {r} 0 0 1 {2*r} 0" fill="none" stroke="#67bbea" opacity=".62"/>')
    for y in range(20, 112, 12):
        arcs.append(f'<path d="M30 {y}H302" stroke="#287c8a" opacity=".29"/>')

    for index, drawing in enumerate((wave, stack, network, arcs)):
        render(f"tile-{index}.png", (320, 105), "".join(drawing))


if __name__ == "__main__":
    atmosphere()
    orbit()
    workspace_accents()
