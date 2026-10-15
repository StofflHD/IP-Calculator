#!/usr/bin/env python3
"""Draws the IP Calculator icon (PNG + SVG) with pycairo, in the style of the notepad-rs icon:
a squircle with the rust gradient (accent colour of the UI theme) and a network that is split
into three subnets, the middle one selected.

Usage: python3 packaging/make_icon.py   (writes packaging/ip-calculator.png/.svg)
"""
import math
import os
import sys

import cairo

HERE = os.path.dirname(os.path.abspath(__file__))

CREAM = "#fbf9f6"
DARK = "#2c2825"
DARKER = "#1b1917"
ACCENT = "#b8411b"


def hexc(h, a=1.0):
    h = h.lstrip("#")
    return (int(h[0:2], 16) / 255, int(h[2:4], 16) / 255, int(h[4:6], 16) / 255, a)


def rounded_rect(cr, x, y, w, h, r):
    cr.new_sub_path()
    cr.arc(x + w - r, y + r, r, -math.pi / 2, 0)
    cr.arc(x + w - r, y + h - r, r, 0, math.pi / 2)
    cr.arc(x + r, y + h - r, r, math.pi / 2, math.pi)
    cr.arc(x + r, y + r, r, math.pi, 3 * math.pi / 2)
    cr.close_path()


def squircle(cr, x, y, s, n=5.0, steps=200):
    """Superellipse like modern app icons."""
    cx, cy, a = x + s / 2, y + s / 2, s / 2
    for i in range(steps + 1):
        t = 2 * math.pi * i / steps
        ct, st = math.cos(t), math.sin(t)
        px = cx + a * math.copysign(abs(ct) ** (2 / n), ct)
        py = cy + a * math.copysign(abs(st) ** (2 / n), st)
        if i == 0:
            cr.move_to(px, py)
        else:
            cr.line_to(px, py)
    cr.close_path()


def network(cr, dx, dy, shadow):
    """Root node, links and the three subnet blocks; `shadow` draws everything flat in black."""
    def fill(colour, a=1.0):
        cr.set_source_rgba(*(0, 0, 0, 0.28) if shadow else hexc(colour, a))

    # links
    cr.set_line_width(12)
    cr.set_line_cap(cairo.LINE_CAP_ROUND)
    cr.set_line_join(cairo.LINE_JOIN_ROUND)
    fill(CREAM)
    cr.move_to(128 + dx, 78 + dy)
    cr.line_to(128 + dx, 172 + dy)
    cr.move_to(58 + dx, 172 + dy)
    cr.line_to(58 + dx, 126 + dy)
    cr.line_to(198 + dx, 126 + dy)
    cr.line_to(198 + dx, 172 + dy)
    cr.stroke()
    # root node
    cr.arc(128 + dx, 70 + dy, 30, 0, 2 * math.pi)
    fill(CREAM)
    cr.fill()
    if not shadow:
        cr.arc(128, 70, 13, 0, 2 * math.pi)
        fill(ACCENT)
        cr.fill()
    # subnets: the selected one is dark with an accent bar, like a selected row
    for i, x in enumerate((58, 128, 198)):
        rounded_rect(cr, x - 27 + dx, 168 + dy, 54, 54, 10)
        fill(DARK if i == 1 else CREAM)
        cr.fill()
    if not shadow:
        cr.save()
        rounded_rect(cr, 101, 168, 54, 54, 10)
        cr.clip()
        cr.rectangle(101, 168, 54, 12)
        fill(ACCENT)
        cr.fill()
        cr.restore()
        # host lines inside the subnets
        cr.set_line_width(6)
        for i, x in enumerate((58, 128, 198)):
            fill(CREAM if i == 1 else "#b9b0a6")
            for j, w in enumerate((30, 22)):
                y = 192 + j * 14
                cr.move_to(x - 15, y)
                cr.line_to(x - 15 + w, y)
                cr.stroke()


def draw(cr, S):
    k = S / 256.0
    cr.scale(k, k)

    # Background: squircle with a rust gradient (accent colour of the UI theme)
    squircle(cr, 8, 8, 240)
    g = cairo.LinearGradient(0, 8, 0, 248)
    g.add_color_stop_rgba(0, *hexc("#f0773a"))
    g.add_color_stop_rgba(1, *hexc("#a8380f"))
    cr.set_source(g)
    cr.fill_preserve()
    cr.set_source_rgba(*hexc("#6b240a", 0.8))
    cr.set_line_width(2)
    cr.stroke()

    # Subtle highlight in the upper half
    cr.save()
    squircle(cr, 8, 8, 240)
    cr.clip()
    hl = cairo.LinearGradient(0, 8, 0, 130)
    hl.add_color_stop_rgba(0, 1, 1, 1, 0.18)
    hl.add_color_stop_rgba(1, 1, 1, 1, 0.0)
    cr.set_source(hl)
    cr.rectangle(0, 0, 256, 130)
    cr.fill()
    cr.restore()

    network(cr, 5, 8, True)
    network(cr, 0, 0, False)


def main():
    png = os.path.join(HERE, "ip-calculator.png")
    svg = os.path.join(HERE, "ip-calculator.svg")
    out = sys.argv[1] if len(sys.argv) > 1 else None
    surf = cairo.ImageSurface(cairo.FORMAT_ARGB32, 256, 256)
    draw(cairo.Context(surf), 256)
    surf.write_to_png(out or png)
    if not out:
        s = cairo.SVGSurface(svg, 256, 256)
        draw(cairo.Context(s), 256)
        s.finish()


if __name__ == "__main__":
    main()
