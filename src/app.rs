//! The calculator window: menus, header, tabs.

use std::collections::HashSet;

use eframe::egui::{
    self, text::LayoutJob, Align, Align2, Color32, FontId, RichText, Sense, Stroke, TextFormat,
    TextStyle, Ui, Vec2,
};
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};

use crate::ipv4::{self, Class, SubnetPlan};
use crate::ipv6;

const HISTORY_LEN: usize = 20;
/// Tables longer than this show a window around the selected row.
const MAX_ROWS: u64 = 1 << 20;
const ROW_H: f32 = 18.0;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
enum Tab {
    Conversions,
    Classes,
    Subnets,
    Cidr,
    Ipv6,
    Info,
}

impl Tab {
    const ALL: [(Tab, &'static str); 6] = [
        (Tab::Conversions, "Conversions"),
        (Tab::Classes, "Classes"),
        (Tab::Subnets, "Subnets"),
        (Tab::Cidr, "CIDR"),
        (Tab::Ipv6, "IPv6"),
        (Tab::Info, "Info"),
    ];
}

/// Everything that survives a restart.
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    allow_one_subnet_bit: bool,
    allow_one_host_bit: bool,
    dark: bool,
    tab: Tab,
    v4: u32,
    v6: String,
    subnet_prefix: u8,
    cidr_block: u8,
    cidr_bits: u8,
    v4_history: Vec<String>,
    v6_history: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            allow_one_subnet_bit: true,
            allow_one_host_bit: true,
            dark: false,
            tab: Tab::Conversions,
            v4: u32::from(std::net::Ipv4Addr::new(10, 20, 0, 0)),
            v6: "::1".into(),
            subnet_prefix: 22,
            cidr_block: 8,
            cidr_bits: 8,
            v4_history: Vec::new(),
            v6_history: Vec::new(),
        }
    }
}

/// Text fields that edit the IPv4 address.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Src {
    Main,
    Decimal,
    DotHex,
    Hex,
    DotBin,
    Bin,
    Other,
}

const CONV: [Src; 5] = [Src::Decimal, Src::DotHex, Src::Hex, Src::DotBin, Src::Bin];

fn render(src: Src, a: u32) -> String {
    match src {
        Src::Main | Src::Other => ipv4::fmt(a),
        Src::Decimal => a.to_string(),
        Src::DotHex => ipv4::dotted_hex(a),
        Src::Hex => ipv4::hex(a),
        Src::DotBin => ipv4::dotted_bin(a),
        Src::Bin => ipv4::bin(a),
    }
}

fn parse_src(src: Src, s: &str) -> Option<u32> {
    match src {
        Src::Main | Src::Other => ipv4::parse(s).map(|(a, _)| a),
        Src::Decimal => ipv4::parse_decimal(s),
        Src::DotHex => ipv4::parse_dotted_hex(s),
        Src::Hex => ipv4::parse_hex(s),
        Src::DotBin => ipv4::parse_dotted_bin(s),
        Src::Bin => ipv4::parse_bin(s),
    }
}

pub struct App {
    s: Settings,
    v4_text: String,
    conv: [String; 5],
    v6: u128,
    v6_text: String,
    collapsed: HashSet<usize>,
    scroll_subnets: bool,
    scroll_cidr: bool,
    scroll_v6: bool,
    about: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let s: Settings = cc
            .storage
            .and_then(|st| eframe::get_value(st, eframe::APP_KEY))
            .unwrap_or_default();
        let v6 = ipv6::parse(&s.v6).unwrap_or(1);
        let mut app = Self {
            v4_text: String::new(),
            conv: Default::default(),
            v6,
            v6_text: s.v6.clone(),
            s,
            collapsed: HashSet::new(),
            scroll_subnets: true,
            scroll_cidr: true,
            scroll_v6: true,
            about: false,
        };
        app.set_v4(app.s.v4, Src::Other);
        app
    }

    fn set_v4(&mut self, a: u32, from: Src) {
        self.s.v4 = a;
        if from != Src::Main {
            self.v4_text = ipv4::fmt(a);
        }
        for (i, src) in CONV.iter().enumerate() {
            if *src != from {
                self.conv[i] = render(*src, a);
            }
        }
        self.scroll_subnets = true;
        self.scroll_cidr = true;
    }

    fn set_v6(&mut self, a: u128, from_text: bool) {
        self.v6 = a;
        if !from_text {
            self.v6_text = ipv6::compressed(a);
        }
        self.s.v6 = ipv6::compressed(a);
        self.scroll_v6 = true;
        // Make sure the matching type is visible in the tree.
        let mut i = ipv6::classify(a);
        while let Some(p) = i.and_then(ipv6::parent) {
            self.collapsed.remove(&p);
            i = Some(p);
        }
    }

    fn apply_style(&self, ctx: &egui::Context) {
        // egui's default greys are soft; text and borders are darkened (light)
        // or brightened (dark) for readability.
        let (mut v, panel, field, stripe, text, strong, border, frame, button) = if self.s.dark {
            (egui::Visuals::dark(), 30, 16, 40, 225, 255, 115, 80, 58)
        } else {
            (egui::Visuals::light(), 240, 255, 230, 20, 0, 115, 150, 225)
        };
        let gray = Color32::from_gray;
        v.panel_fill = gray(panel);
        v.window_fill = gray(panel);
        v.extreme_bg_color = gray(field);
        v.faint_bg_color = gray(stripe);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0f32, gray(text));
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, gray(frame));
        v.widgets.inactive.fg_stroke = Stroke::new(1.0f32, gray(text));
        v.widgets.inactive.bg_stroke = Stroke::new(1.0f32, gray(border));
        v.widgets.inactive.bg_fill = gray(button);
        v.widgets.inactive.weak_bg_fill = gray(button);
        v.widgets.hovered.fg_stroke = Stroke::new(1.5f32, gray(strong));
        v.widgets.active.fg_stroke = Stroke::new(2.0f32, gray(strong));
        v.widgets.hovered.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(0, 120, 215));
        v.error_fg_color = tone(self.s.dark, (185, 0, 0), (255, 120, 120));
        v.selection.bg_fill = Color32::from_rgb(0, 102, 204);
        v.selection.stroke = Stroke::new(1.0f32, Color32::WHITE);
        ctx.set_visuals(v);
    }

    // ------------------------------------------------------------- widgets

    /// Address entry with a drop-down of previously entered addresses.
    fn address_field(
        ui: &mut Ui,
        text: &mut String,
        valid: bool,
        history: &mut Vec<String>,
    ) -> (egui::Response, Option<String>) {
        let mut picked = None;
        let resp = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let mut edit =
                    egui::TextEdit::singleline(text).desired_width(ui.available_width() - 26.0);
                if !valid {
                    edit = edit.text_color(ui.visuals().error_fg_color);
                }
                let resp = ui.add(edit);
                ui.menu_button("⏷", |ui| {
                    if history.is_empty() {
                        ui.label(RichText::new("(empty)").weak());
                    }
                    for h in history.iter() {
                        if ui.button(h).clicked() {
                            picked = Some(h.clone());
                            ui.close_menu();
                        }
                    }
                    if !history.is_empty() {
                        ui.separator();
                        if ui.button("Clear History").clicked() {
                            history.clear();
                            ui.close_menu();
                        }
                    }
                });
                resp
            })
            .inner;
        if resp.lost_focus() && valid {
            remember(history, text.trim());
        }
        (resp, picked)
    }

    /// The shared IPv4 address entry; returns the "/prefix" if one was typed.
    fn v4_address(&mut self, ui: &mut Ui, label: &str) -> Option<u8> {
        field_label(ui, label);
        let valid = ipv4::parse(&self.v4_text).is_some();
        let (resp, picked) =
            Self::address_field(ui, &mut self.v4_text, valid, &mut self.s.v4_history);
        let input = match picked {
            Some(p) => {
                self.v4_text = p;
                true
            }
            None => resp.changed(),
        };
        if input {
            if let Some((a, prefix)) = ipv4::parse(&self.v4_text) {
                self.set_v4(a, Src::Main);
                return prefix;
            }
        }
        None
    }

    fn conv_field(&mut self, ui: &mut Ui, label: &str, src: Src) {
        let i = CONV.iter().position(|s| *s == src).unwrap();
        field_label(ui, label);
        let valid = parse_src(src, &self.conv[i]).is_some();
        let mut edit = egui::TextEdit::singleline(&mut self.conv[i]).desired_width(f32::INFINITY);
        if matches!(src, Src::DotBin | Src::Bin) {
            edit = edit.font(TextStyle::Monospace);
        }
        if !valid {
            edit = edit.text_color(ui.visuals().error_fg_color);
        }
        let resp = ui.add(edit);
        if resp.changed() {
            if let Some(a) = parse_src(src, &self.conv[i]) {
                self.set_v4(a, src);
            }
        }
        if resp.lost_focus() {
            self.conv[i] = render(src, self.s.v4);
        }
    }

    // ---------------------------------------------------------------- tabs

    fn tab_conversions(&mut self, ui: &mut Ui) {
        two_columns(ui, |ui, col| match col {
            0 => {
                self.v4_address(ui, "Dotted Decimal");
            }
            _ => {
                self.conv_field(ui, "Decimal", Src::Decimal);
            }
        });
        two_columns(ui, |ui, col| match col {
            0 => {
                self.conv_field(ui, "Dotted Hexadecimal", Src::DotHex);
            }
            _ => {
                self.conv_field(ui, "Hexadecimal", Src::Hex);
            }
        });
        self.conv_field(ui, "Dotted Binary", Src::DotBin);
        self.conv_field(ui, "Binary", Src::Bin);
        let mac = ipv4::multicast_mac(self.s.v4);
        ui.add_enabled_ui(mac.is_some(), |ui| {
            field_label(ui, "Ethernet Multicast");
            readonly(ui, &mac.unwrap_or_default(), true);
        });
    }

    fn tab_classes(&mut self, ui: &mut Ui) {
        let class = Class::of(self.s.v4);
        two_columns(ui, |ui, col| match col {
            0 => {
                if let Some(p) = self.v4_address(ui, "Address") {
                    self.s.subnet_prefix = p;
                }
            }
            _ => {
                field_label(ui, "Class");
                let mut sel = class;
                egui::ComboBox::from_id_salt("class")
                    .width(ui.available_width())
                    .selected_text(class.name())
                    .show_ui(ui, |ui| {
                        for c in Class::ALL {
                            ui.selectable_value(&mut sel, c, c.name());
                        }
                    });
                if sel != class {
                    self.set_v4(sel.range().0, Src::Other);
                }
            }
        });
        let (lo, hi) = class.range();
        field_label(ui, "Class Address Range");
        readonly(ui, &format!("{} - {}", ipv4::fmt(lo), ipv4::fmt(hi)), false);
        let legend: &[(&str, char)] = match class {
            Class::D => &[("m", 'm'), ("=Multicast Group", ' ')],
            Class::E => &[("r", 'r'), ("=Reserved", ' ')],
            _ => &[("n", 'n'), ("=Network; ", ' '), ("h", 'h'), ("=Host", ' ')],
        };
        legend_label(ui, "Class Bit Usage", legend);
        bit_usage(ui, &class.bit_usage(self.s.v4));
        ui.add_space(8.0);
        let p = self.s.v4 & ipv4::mask(class.network_bits().unwrap_or(4));
        let net = match class.network_bits() {
            Some(n) => format!("{}/{}  ({})", ipv4::fmt(p), n, ipv4::fmt_mask(n)),
            None => "classless".into(),
        };
        field_label(ui, "Classful Network");
        readonly(ui, &net, false);
    }

    /// Prefix lengths selectable in the subnets tab for classful network bits `n`.
    fn subnet_prefixes(&self, n: u8) -> Vec<u8> {
        let min_s = if self.s.allow_one_subnet_bit { 1 } else { 2 };
        let min_h = if self.s.allow_one_host_bit { 1 } else { 2 };
        std::iter::once(n).chain(n + min_s..=32 - min_h).collect()
    }

    fn tab_subnets(&mut self, ui: &mut Ui) {
        let class = Class::of(self.s.v4);
        let n = class.network_bits();
        two_columns(ui, |ui, col| match col {
            0 => {
                if let Some(p) = self.v4_address(ui, "Address") {
                    self.s.subnet_prefix = p;
                    self.scroll_subnets = true;
                }
            }
            _ => {
                field_label(ui, "Subnet Mask");
                ui.add_enabled_ui(n.is_some(), |ui| {
                    let prefixes = n.map(|n| self.subnet_prefixes(n)).unwrap_or_default();
                    let cur = effective(self.s.subnet_prefix, &prefixes);
                    if let Some(p) = combo(
                        ui,
                        "mask",
                        cur,
                        prefixes.iter().map(|&p| (p, ipv4::fmt_mask(p))),
                    ) {
                        self.s.subnet_prefix = p;
                        self.scroll_subnets = true;
                    }
                });
            }
        });
        let Some(n) = n else {
            ui.add_space(12.0);
            ui.label(format!(
                "{} addresses are not subnetted. Use the CIDR tab instead.",
                class.name()
            ));
            return;
        };
        let prefixes = self.subnet_prefixes(n);
        let p = effective(self.s.subnet_prefix, &prefixes);
        let rfc1878 = self.s.allow_one_subnet_bit;
        let mut new_p = None;
        ui.columns(4, |cols| {
            field_label(&mut cols[0], "Subnet Bits");
            new_p = new_p.or(combo(
                &mut cols[0],
                "sbits",
                p,
                prefixes.iter().map(|&q| (q, (q - n).to_string())),
            ));
            field_label(&mut cols[1], "Max Subnets");
            new_p = new_p.or(combo(
                &mut cols[1],
                "msub",
                p,
                prefixes
                    .iter()
                    .map(|&q| (q, group(ipv4::max_subnets(q - n, rfc1878)))),
            ));
            field_label(&mut cols[2], "Host Bits");
            new_p = new_p.or(combo(
                &mut cols[2],
                "hbits",
                p,
                prefixes.iter().map(|&q| (q, (32 - q).to_string())),
            ));
            field_label(&mut cols[3], "Max Hosts");
            new_p = new_p.or(combo(
                &mut cols[3],
                "mhost",
                p,
                prefixes
                    .iter()
                    .map(|&q| (q, group(ipv4::max_hosts(32 - q)))),
            ));
        });
        if let Some(q) = new_p {
            self.s.subnet_prefix = q;
            self.scroll_subnets = true;
        }

        legend_label(
            ui,
            "Subnet Bit Usage",
            &[
                ("n", 'n'),
                ("=Network; ", ' '),
                ("s", 's'),
                ("=Subnet; ", ' '),
                ("h", 'h'),
                ("=Host", ' '),
            ],
        );
        let a = self.s.v4;
        let lead = class.lead_bits() as u32;
        bit_usage(
            ui,
            &ipv4::pattern(|i| match i {
                _ if i < lead => bit(a, i),
                _ if i < n as u32 => 'n',
                _ if i < p as u32 => 's',
                _ => 'h',
            }),
        );

        let plan = SubnetPlan {
            base: a & ipv4::mask(n),
            net_bits: n,
            prefix: p,
            skip_edges: !rfc1878,
        };
        field_label(ui, "Subnets/Address Allocations");
        if plan.count() == 0 {
            ui.label("No usable subnets.");
            return;
        }
        let sel = plan
            .index_of(a)
            .checked_sub(plan.first_index())
            .filter(|&r| r < plan.count());
        let (offset, rows) = window(plan.count(), sel, MAX_ROWS);
        let scroll = std::mem::take(&mut self.scroll_subnets);
        let mut clicked = None;
        table_frame(ui, |ui| {
            let mut tb = TableBuilder::new(ui)
                .id_salt("subnets")
                .striped(true)
                .sense(Sense::click())
                .auto_shrink(false)
                .column(Column::exact(48.0))
                .column(Column::exact(100.0))
                .column(Column::remainder().at_least(180.0))
                .column(Column::exact(115.0));
            if let (true, Some(s)) = (scroll, sel) {
                tb = tb.scroll_to_row((s - offset) as usize, Some(Align::Center));
            }
            tb.header(20.0, |mut h| {
                for t in ["", "Subnet ID", "Host Addresses", "Subnet Broadcast"] {
                    h.col(|ui| {
                        ui.strong(t);
                    });
                }
            })
            .body(|body| {
                body.rows(ROW_H, rows as usize, |mut row| {
                    let r = offset + row.index() as u64;
                    let idx = plan.first_index() + r;
                    let sub = plan.subnet(idx);
                    let selected = sel == Some(r);
                    row.set_selected(selected);
                    let hosts = format!("{} - {}", ipv4::fmt(sub.first), ipv4::fmt(sub.last));
                    let bcast = sub.broadcast.map_or("—".to_string(), ipv4::fmt);
                    cell(&mut row, &idx.to_string(), selected, true);
                    cell(&mut row, &ipv4::fmt(sub.id), selected, false);
                    cell(&mut row, &hosts, selected, false);
                    cell(&mut row, &bcast, selected, false);
                    let resp = row.response();
                    if resp.clicked() {
                        clicked = Some(sub.id);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Copy Row").clicked() {
                            ui.ctx().copy_text(format!(
                                "{}\t{}\t{}\t{}",
                                idx,
                                ipv4::fmt(sub.id),
                                hosts,
                                bcast
                            ));
                            ui.close_menu();
                        }
                        if ui.button("Copy Subnet (CIDR)").clicked() {
                            ui.ctx().copy_text(format!("{}/{}", ipv4::fmt(sub.id), p));
                            ui.close_menu();
                        }
                    });
                });
            });
        });
        if let Some(id) = clicked {
            self.set_v4(id, Src::Other);
            self.scroll_subnets = false;
        }
    }

    fn tab_cidr(&mut self, ui: &mut Ui) {
        two_columns(ui, |ui, col| match col {
            0 => {
                if let Some(p) = self.v4_address(ui, "Address") {
                    self.s.cidr_block = p.max(1);
                    self.scroll_cidr = true;
                }
            }
            _ => {
                field_label(ui, "Address Block Mask");
                if let Some(b) = combo(
                    ui,
                    "block",
                    self.s.cidr_block,
                    (1..=32).map(|p| (p, ipv4::fmt_mask(p))),
                ) {
                    self.s.cidr_block = b;
                    self.scroll_cidr = true;
                }
            }
        });
        let a = self.s.v4;
        let block = self.s.cidr_block.clamp(1, 32);
        let bits = self.s.cidr_bits.min(32 - block);
        let (start, end) = ipv4::range(a, block);
        field_label(ui, "Address Block Range");
        readonly(
            ui,
            &format!("{} - {}", ipv4::fmt(start), ipv4::fmt(end)),
            false,
        );

        let mut new_bits = None;
        ui.columns(3, |cols| {
            field_label(&mut cols[0], "CIDR Bits");
            new_bits = new_bits.or(combo(
                &mut cols[0],
                "cbits",
                bits,
                (0..=32 - block).map(|c| (c, c.to_string())),
            ));
            field_label(&mut cols[1], "Max Routes");
            new_bits = new_bits.or(combo(
                &mut cols[1],
                "routes",
                bits,
                (0..=32 - block).map(|c| (c, group(1u64 << c))),
            ));
            field_label(&mut cols[2], "CIDR Mask");
            new_bits = new_bits.or(combo(
                &mut cols[2],
                "cmask",
                bits,
                (0..=32 - block).map(|c| (c, ipv4::fmt_mask(block + c))),
            ));
        });
        if let Some(c) = new_bits {
            self.s.cidr_bits = c;
            self.scroll_cidr = true;
        }

        legend_label(
            ui,
            "CIDR Bit Usage",
            &[("c", 'c'), ("=CIDR; ", ' '), ("x", 'x'), ("=Open", ' ')],
        );
        let route_p = block + bits;
        bit_usage(
            ui,
            &ipv4::pattern(|i| match i {
                _ if i < block as u32 => bit(a, i),
                _ if i < route_p as u32 => 'c',
                _ => 'x',
            }),
        );

        field_label(ui, "Routes/Address Allocations");
        let shift = 32 - route_p as u32;
        let count = 1u64 << bits;
        let sel = Some(((a - start) as u64) >> shift);
        let (offset, rows) = window(count, sel, MAX_ROWS);
        let scroll = std::mem::take(&mut self.scroll_cidr);
        let mut clicked = None;
        table_frame(ui, |ui| {
            let mut tb = TableBuilder::new(ui)
                .id_salt("routes")
                .striped(true)
                .sense(Sense::click())
                .auto_shrink(false)
                .column(Column::exact(58.0))
                .column(Column::exact(110.0))
                .column(Column::remainder());
            if let (true, Some(s)) = (scroll, sel) {
                tb = tb.scroll_to_row((s - offset) as usize, Some(Align::Center));
            }
            tb.header(20.0, |mut h| {
                for t in ["", "Route", "Address Range"] {
                    h.col(|ui| {
                        ui.strong(t);
                    });
                }
            })
            .body(|body| {
                body.rows(ROW_H, rows as usize, |mut row| {
                    let r = offset + row.index() as u64;
                    let route = start + ((r as u32).checked_shl(shift).unwrap_or(0));
                    let last = route | !ipv4::mask(route_p);
                    let selected = sel == Some(r);
                    row.set_selected(selected);
                    let range = format!("{} - {}", ipv4::fmt(route), ipv4::fmt(last));
                    cell(&mut row, &r.to_string(), selected, true);
                    cell(&mut row, &ipv4::fmt(route), selected, false);
                    cell(&mut row, &range, selected, false);
                    let resp = row.response();
                    if resp.clicked() {
                        clicked = Some(route);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Copy Route (CIDR)").clicked() {
                            ui.ctx()
                                .copy_text(format!("{}/{}", ipv4::fmt(route), route_p));
                            ui.close_menu();
                        }
                        if ui.button("Copy Address Range").clicked() {
                            ui.ctx().copy_text(range.clone());
                            ui.close_menu();
                        }
                    });
                });
            });
        });
        if let Some(r) = clicked {
            self.set_v4(r, Src::Other);
            self.scroll_cidr = false;
        }
    }

    fn tab_ipv6(&mut self, ui: &mut Ui) {
        field_label(ui, "Address");
        let valid = ipv6::parse(&self.v6_text).is_some();
        let (resp, picked) =
            Self::address_field(ui, &mut self.v6_text, valid, &mut self.s.v6_history);
        let input = match picked {
            Some(p) => {
                self.v6_text = p;
                true
            }
            None => resp.changed(),
        };
        if input {
            if let Some(a) = ipv6::parse(&self.v6_text) {
                self.set_v6(a, true);
            }
        }

        let ty = ipv6::classify(self.v6);
        field_label(ui, "Address Type");
        let visible: Vec<usize> = (0..ipv6::TYPES.len())
            .filter(|&i| {
                let mut p = ipv6::parent(i);
                while let Some(j) = p {
                    if self.collapsed.contains(&j) {
                        return false;
                    }
                    p = ipv6::parent(j);
                }
                true
            })
            .collect();
        let scroll = std::mem::take(&mut self.scroll_v6);
        let mut action = None;
        table_frame_h(ui, 172.0, |ui| {
            let mut tb = TableBuilder::new(ui)
                .id_salt("v6types")
                .sense(Sense::click())
                .auto_shrink(false)
                .max_scroll_height(150.0)
                .column(Column::remainder().at_least(160.0))
                .column(Column::exact(130.0))
                .column(Column::exact(82.0));
            if let (true, Some(r)) = (
                scroll,
                ty.and_then(|t| visible.iter().position(|&v| v == t)),
            ) {
                tb = tb.scroll_to_row(r, Some(Align::Center));
            }
            tb.header(20.0, |mut h| {
                for t in ["Name", "Pattern", "Notes"] {
                    h.col(|ui| {
                        ui.strong(t);
                    });
                }
            })
            .body(|body| {
                body.rows(ROW_H, visible.len(), |mut row| {
                    let i = visible[row.index()];
                    let t = &ipv6::TYPES[i];
                    let selected = ty == Some(i);
                    row.set_selected(selected);
                    let weak = t.is_abstract() && !selected;
                    let color = |ui: &Ui| {
                        if selected {
                            Color32::WHITE
                        } else if weak {
                            ui.visuals().weak_text_color()
                        } else {
                            ui.visuals().text_color()
                        }
                    };
                    let mut arrow = egui::Rect::NOTHING;
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.add_space(t.depth as f32 * 14.0);
                            let c = color(ui);
                            if ipv6::has_children(i) {
                                let a = if self.collapsed.contains(&i) {
                                    "⏵"
                                } else {
                                    "⏷"
                                };
                                arrow = ui
                                    .add_sized(
                                        [16.0, ROW_H],
                                        egui::Label::new(RichText::new(a).color(c))
                                            .selectable(false),
                                    )
                                    .rect
                                    .expand(2.0);
                            } else {
                                ui.add_space(16.0);
                            }
                            ui.add(
                                egui::Label::new(RichText::new(t.name).color(c)).selectable(false),
                            );
                        });
                    });
                    row.col(|ui| {
                        let c = color(ui);
                        ui.add(
                            egui::Label::new(RichText::new(t.pattern).color(c)).selectable(false),
                        );
                    });
                    row.col(|ui| {
                        let c = color(ui);
                        ui.add(egui::Label::new(RichText::new(t.note).color(c)).selectable(false));
                    });
                    let resp = row.response();
                    if resp.clicked() {
                        let on_arrow = resp
                            .interact_pointer_pos()
                            .is_some_and(|p| arrow.contains(p));
                        action = Some((i, on_arrow));
                    }
                    if resp.double_clicked() && ipv6::has_children(i) {
                        action = Some((i, true));
                    }
                });
            });
        });
        match action {
            Some((i, true)) if ipv6::has_children(i) => {
                if !self.collapsed.remove(&i) {
                    self.collapsed.insert(i);
                }
            }
            Some((i, _)) => {
                if let Some((net, _)) = ipv6::TYPES[i].prefix() {
                    self.set_v6(net, false);
                }
            }
            None => {}
        }

        field_label(ui, "Address Format");
        egui::Frame::group(ui.style())
            .fill(ui.visuals().extreme_bg_color)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                format_bar(
                    ui,
                    &ipv6::fields(self.v6, ty),
                    ty.map_or("Unknown", |i| ipv6::TYPES[i].name),
                );
            });
        field_label(ui, "Expanded");
        readonly(ui, &ipv6::expanded(self.v6), true);
        field_label(ui, "Reverse DNS");
        readonly(ui, &ipv6::reverse_dns(self.v6), true);
    }

    fn tab_info(&mut self, ui: &mut Ui) {
        let a = self.s.v4;
        two_columns(ui, |ui, col| match col {
            0 => {
                if let Some(p) = self.v4_address(ui, "Address") {
                    self.s.subnet_prefix = p;
                }
            }
            _ => {
                field_label(ui, "Class");
                readonly(ui, Class::of(a).name(), false);
            }
        });
        let sp = ipv4::special_of(a);
        field_label(ui, "Address Space");
        readonly(
            ui,
            &sp.map_or("Public (globally routable)".to_string(), |i| {
                format!("{} ({})", ipv4::SPECIAL[i].name, ipv4::SPECIAL[i].rfc)
            }),
            false,
        );
        two_columns(ui, |ui, col| match col {
            0 => {
                field_label(ui, "IPv4-Mapped IPv6");
                readonly(ui, &format!("::ffff:{}", ipv4::fmt(a)), false);
            }
            _ => {
                field_label(ui, "6to4 Prefix");
                let b = a.to_be_bytes();
                readonly(
                    ui,
                    &format!(
                        "2002:{:02x}{:02x}:{:02x}{:02x}::/48",
                        b[0], b[1], b[2], b[3]
                    ),
                    false,
                );
            }
        });
        field_label(ui, "Reverse DNS");
        readonly(ui, &ipv4::reverse_dns(a), false);

        field_label(ui, "Special-Purpose Address Blocks");
        table_frame(ui, |ui| {
            TableBuilder::new(ui)
                .id_salt("special")
                .striped(true)
                .auto_shrink(false)
                .column(Column::exact(130.0))
                .column(Column::remainder())
                .column(Column::exact(72.0))
                .header(20.0, |mut h| {
                    for t in ["Block", "Name", "Reference"] {
                        h.col(|ui| {
                            ui.strong(t);
                        });
                    }
                })
                .body(|body| {
                    body.rows(ROW_H, ipv4::SPECIAL.len(), |mut row| {
                        let i = row.index();
                        let s = &ipv4::SPECIAL[i];
                        let selected = sp == Some(i);
                        row.set_selected(selected);
                        cell(&mut row, s.block, selected, false);
                        cell(&mut row, s.name, selected, false);
                        cell(&mut row, s.rfc, selected, false);
                    });
                });
        });
    }

    // ----------------------------------------------------- chrome & export

    fn menu(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Copy Subnets as CSV").clicked() {
                    ui.ctx().copy_text(self.subnets_csv());
                    ui.close_menu();
                }
                if ui.button("Copy Routes as CSV").clicked() {
                    ui.ctx().copy_text(self.routes_csv());
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Options", |ui| {
                ui.checkbox(
                    &mut self.s.allow_one_subnet_bit,
                    "Allow 1 Subnet Bit (RFC 1878)",
                );
                ui.checkbox(
                    &mut self.s.allow_one_host_bit,
                    "Allow 1 Host Bit (RFC 3021)",
                );
                ui.separator();
                ui.checkbox(&mut self.s.dark, "Dark Theme");
            });
            ui.menu_button("Help", |ui| {
                if ui.button("About IP Calculator").clicked() {
                    self.about = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn subnets_csv(&self) -> String {
        let a = self.s.v4;
        let Some(n) = Class::of(a).network_bits() else {
            return String::new();
        };
        let p = effective(self.s.subnet_prefix, &self.subnet_prefixes(n));
        let plan = SubnetPlan {
            base: a & ipv4::mask(n),
            net_bits: n,
            prefix: p,
            skip_edges: !self.s.allow_one_subnet_bit,
        };
        let mut out = String::from("index,subnet_id,prefix,first_host,last_host,broadcast\n");
        for idx in plan.first_index()..plan.first_index() + plan.count().min(MAX_ROWS) {
            let s = plan.subnet(idx);
            let b = s.broadcast.map(ipv4::fmt).unwrap_or_default();
            out += &format!(
                "{idx},{},{p},{},{},{b}\n",
                ipv4::fmt(s.id),
                ipv4::fmt(s.first),
                ipv4::fmt(s.last)
            );
        }
        out
    }

    fn routes_csv(&self) -> String {
        let block = self.s.cidr_block.clamp(1, 32);
        let route_p = block + self.s.cidr_bits.min(32 - block);
        let (start, _) = ipv4::range(self.s.v4, block);
        let mut out = String::from("index,route,first,last\n");
        for r in 0..(1u64 << (route_p - block)).min(MAX_ROWS) {
            let route = start + (r as u32).checked_shl(32 - route_p as u32).unwrap_or(0);
            let (first, last) = ipv4::range(route, route_p);
            out += &format!(
                "{r},{}/{route_p},{},{}\n",
                ipv4::fmt(route),
                ipv4::fmt(first),
                ipv4::fmt(last)
            );
        }
        out
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("About IP Calculator")
            .open(&mut self.about)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(format!("IP Calculator {}", env!("CARGO_PKG_VERSION")));
                    ui.label("IPv4/IPv6 subnet calculator written in Rust.");
                    ui.add_space(10.0);
                    ui.separator();
                    ui.label(RichText::new(env!("CARGO_PKG_AUTHORS")).strong());
                });
            });
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.s);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_style(ctx);
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            header(ui);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (tab, name) in Tab::ALL {
                    let sel = self.s.tab == tab;
                    let text = RichText::new(name);
                    if ui
                        .add(egui::SelectableLabel::new(
                            sel,
                            if sel { text.strong() } else { text },
                        ))
                        .clicked()
                        && !sel
                    {
                        self.s.tab = tab;
                        self.scroll_subnets = true;
                        self.scroll_cidr = true;
                        self.scroll_v6 = true;
                    }
                }
            });
            egui::Frame::group(ui.style())
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(ui.available_height() - 6.0);
                    match self.s.tab {
                        Tab::Conversions => self.tab_conversions(ui),
                        Tab::Classes => self.tab_classes(ui),
                        Tab::Subnets => self.tab_subnets(ui),
                        Tab::Cidr => self.tab_cidr(ui),
                        Tab::Ipv6 => self.tab_ipv6(ui),
                        Tab::Info => self.tab_info(ui),
                    }
                });
        });
        self.about_window(ctx);
    }
}

// ------------------------------------------------------------------ helpers

/// Uses DejaVu Sans for proportional text; egui's built-in font is a thin
/// light weight. The built-in fonts stay as fallback (symbols, emoji).
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "DejaVuSans".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/DejaVuSans.ttf")),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "DejaVuSans".into());
    ctx.set_fonts(fonts);
}

fn remember(history: &mut Vec<String>, entry: &str) {
    if entry.is_empty() {
        return;
    }
    history.retain(|h| h != entry);
    history.insert(0, entry.to_string());
    history.truncate(HISTORY_LEN);
}

/// Snaps a stored prefix to the nearest selectable one.
fn effective(stored: u8, valid: &[u8]) -> u8 {
    valid
        .iter()
        .copied()
        .min_by_key(|&p| (p as i16 - stored as i16).abs())
        .unwrap_or(stored)
}

/// Visible slice of a long table: (first row, row count).
fn window(count: u64, sel: Option<u64>, limit: u64) -> (u64, u64) {
    if count <= limit {
        return (0, count);
    }
    let center = sel.unwrap_or(0);
    let start = center.saturating_sub(limit / 2).min(count - limit);
    (start, limit)
}

fn bit(a: u32, i: u32) -> char {
    if a & (1 << (31 - i)) != 0 {
        '1'
    } else {
        '0'
    }
}

/// 16384 -> "16,384"
fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn field_label(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(text);
}

fn readonly(ui: &mut Ui, text: &str, mono: bool) {
    let mut t = text;
    let mut edit = egui::TextEdit::singleline(&mut t).desired_width(f32::INFINITY);
    if mono {
        edit = edit.font(TextStyle::Monospace);
    }
    ui.add(edit);
}

fn two_columns(ui: &mut Ui, mut add: impl FnMut(&mut Ui, usize)) {
    ui.columns(2, |cols| {
        for (i, col) in cols.iter_mut().enumerate() {
            col.vertical(|ui| add(ui, i));
        }
    });
}

fn combo<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: &str,
    current: T,
    items: impl Iterator<Item = (T, String)>,
) -> Option<T> {
    let items: Vec<(T, String)> = items.collect();
    let text = items
        .iter()
        .find(|(v, _)| *v == current)
        .map(|(_, t)| t.clone())
        .unwrap_or_default();
    let mut sel = current;
    egui::ComboBox::from_id_salt(id)
        .width(ui.available_width())
        .selected_text(text)
        .height(320.0)
        .show_ui(ui, |ui| {
            for (v, t) in items {
                ui.selectable_value(&mut sel, v, t);
            }
        });
    (sel != current).then_some(sel)
}

/// Picks the light- or dark-theme variant of a colour.
fn tone(dark: bool, light: (u8, u8, u8), dark_rgb: (u8, u8, u8)) -> Color32 {
    let (r, g, b) = if dark { dark_rgb } else { light };
    Color32::from_rgb(r, g, b)
}

fn bit_color(c: char, ui: &Ui) -> Color32 {
    let dark = ui.visuals().dark_mode;
    match c {
        's' | 'c' => tone(dark, (0, 70, 190), (130, 185, 255)),
        'h' => tone(dark, (0, 115, 45), (110, 215, 140)),
        'm' => tone(dark, (135, 45, 175), (215, 160, 245)),
        'x' | 'r' => ui.visuals().weak_text_color(),
        _ => ui.visuals().strong_text_color(),
    }
}

/// "Label (n=Network; h=Host)" with the letters in their bit colours.
fn legend_label(ui: &mut Ui, title: &str, parts: &[(&str, char)]) {
    ui.add_space(4.0);
    let font = TextStyle::Body.resolve(ui.style());
    let normal = TextFormat::simple(font.clone(), ui.visuals().text_color());
    let mut job = LayoutJob::default();
    job.append(&format!("{title} ("), 0.0, normal.clone());
    for (text, c) in parts {
        let fmt = if *c == ' ' {
            normal.clone()
        } else {
            TextFormat::simple(font.clone(), bit_color(*c, ui))
        };
        job.append(text, 0.0, fmt);
    }
    job.append(")", 0.0, normal);
    ui.label(job);
}

/// Read-only, text-field-like display of a coloured bit pattern.
fn bit_usage(ui: &mut Ui, pattern: &str) {
    let font = FontId::monospace(13.0);
    let mut job = LayoutJob::default();
    for ch in pattern.chars() {
        job.append(
            &ch.to_string(),
            0.0,
            TextFormat::simple(font.clone(), bit_color(ch, ui)),
        );
    }
    egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .inner_margin(egui::Margin::symmetric(4.0, 3.0))
        .rounding(2.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(job);
        });
}

fn table_frame(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let h = ui.available_height() - 4.0;
    table_frame_h(ui, h, add);
}

fn table_frame_h(ui: &mut Ui, height: f32, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .inner_margin(2.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(height.max(80.0));
            add(ui);
        });
}

fn cell(row: &mut egui_extras::TableRow<'_, '_>, text: &str, selected: bool, index: bool) {
    row.col(|ui| {
        let mut t = RichText::new(text);
        if selected {
            t = t.color(Color32::WHITE);
        } else if index {
            t = t.color(tone(ui.visuals().dark_mode, (165, 20, 20), (255, 130, 130)));
        }
        if index {
            t = t.strong();
        }
        ui.add(egui::Label::new(t).selectable(false).truncate());
    });
}

const LOGO_BG: Color32 = Color32::from_rgb(31, 78, 121);
const LOGO_LINE: Color32 = Color32::from_rgb(156, 195, 230);
const LOGO_ACCENT: Color32 = Color32::from_rgb(245, 166, 35);

/// Paints the logo (a network split into three subnets, one selected) into a
/// square of side `size` at `min`. Drawn on a 32×32 grid, like `packaging/ip-calculator.svg`.
fn paint_logo(p: &egui::Painter, min: egui::Pos2, size: f32) {
    let k = size / 32.0;
    let at = |x: f32, y: f32| min + Vec2::new(x, y) * k;
    let square = |x: f32, y: f32| egui::Rect::from_center_size(at(x, y), Vec2::splat(7.0 * k));
    p.rect_filled(
        egui::Rect::from_min_size(min, Vec2::splat(size)),
        7.0 * k,
        LOGO_BG,
    );
    let line = Stroke::new(1.8 * k, LOGO_LINE);
    p.line_segment([at(16.0, 8.5), at(16.0, 23.5)], line);
    p.line_segment([at(7.0, 15.5), at(25.0, 15.5)], line);
    p.line_segment([at(7.0, 15.5), at(7.0, 23.5)], line);
    p.line_segment([at(25.0, 15.5), at(25.0, 23.5)], line);
    p.circle_filled(at(16.0, 8.5), 3.8 * k, Color32::WHITE);
    p.rect_filled(square(7.0, 23.5), 1.5 * k, LOGO_LINE);
    p.rect_filled(square(16.0, 23.5), 1.5 * k, LOGO_ACCENT);
    p.rect_filled(square(25.0, 23.5), 1.5 * k, LOGO_LINE);
}

/// Logo and subtitle, drawn rather than loaded from an image.
fn header(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 86.0), Sense::hover());
    let p = ui.painter_at(rect);
    let logo = FontId::proportional(40.0);
    let grey = if ui.visuals().dark_mode {
        Color32::from_gray(225)
    } else {
        Color32::from_gray(55)
    };
    // Blue of the logo family; the dark variant would vanish on a dark panel.
    let blue = if ui.visuals().dark_mode {
        LOGO_LINE
    } else {
        Color32::from_rgb(46, 117, 182)
    };
    let galley_ip = p.layout_no_wrap("ip".into(), logo.clone(), blue);
    let galley_calc = p.layout_no_wrap("calc".into(), logo, grey);
    let icon = 54.0;
    let gap = 10.0;
    let w = icon + gap + galley_ip.size().x + galley_calc.size().x;
    let x = rect.center().x - w / 2.0;
    paint_logo(&p, egui::pos2(x, rect.top() + 4.0), icon);
    let tx = x + icon + gap;
    let y = rect.top() + 8.0;
    p.galley(egui::pos2(tx, y), galley_ip.clone(), blue);
    p.galley(egui::pos2(tx + galley_ip.size().x, y), galley_calc, grey);
    p.text(
        egui::pos2(rect.center().x, rect.bottom() - 10.0),
        Align2::CENTER_CENTER,
        "IPv4 / IPv6 Subnet Calculator",
        FontId::proportional(16.0),
        ui.visuals().strong_text_color(),
    );
}

/// The segmented 128-bit bar of the IPv6 tab.
fn format_bar(ui: &mut Ui, fields: &[ipv6::Field], type_name: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 70.0), Sense::hover());
    let p = ui.painter_at(rect);
    let weights: Vec<f32> = fields.iter().map(|f| (f.bits as f32).max(18.0)).collect();
    let total: f32 = weights.iter().sum();
    let dark = ui.visuals().dark_mode;
    let small = FontId::proportional(11.0);
    let text = ui.visuals().text_color();
    let label_c = tone(dark, (0, 70, 190), (130, 185, 255));
    let value_c = tone(dark, (165, 45, 0), (255, 175, 115));
    let bar_top = rect.top() + 16.0;
    let bar_h = 22.0;
    let mut x = rect.left() + 2.0;
    let width = rect.width() - 4.0;
    for (f, w) in fields.iter().zip(&weights) {
        let seg =
            egui::Rect::from_min_size(egui::pos2(x, bar_top), Vec2::new(width * w / total, bar_h));
        let clip = p.with_clip_rect(seg.expand2(Vec2::new(-1.0, 20.0)).intersect(rect));
        clip.text(
            egui::pos2(seg.center().x, rect.top() + 7.0),
            Align2::CENTER_CENTER,
            format!("{} bits", f.bits),
            small.clone(),
            text,
        );
        p.rect(
            seg.shrink2(Vec2::new(1.0, 0.0)),
            3.0,
            Color32::from_gray(if dark { 52 } else { 226 }),
            Stroke::new(1.0f32, Color32::from_gray(if dark { 130 } else { 115 })),
        );
        clip.text(
            seg.center(),
            Align2::CENTER_CENTER,
            &f.value,
            FontId::monospace(11.0),
            value_c,
        );
        clip.text(
            egui::pos2(seg.center().x, seg.bottom() + 9.0),
            Align2::CENTER_CENTER,
            &f.label,
            small.clone(),
            label_c,
        );
        ui.interact(seg, ui.id().with(x as i32), Sense::hover())
            .on_hover_text(format!("{}: {} ({} bits)", f.label, f.value, f.bits));
        x += seg.width();
    }
    p.text(
        egui::pos2(rect.center().x, rect.bottom() - 6.0),
        Align2::CENTER_CENTER,
        type_name,
        small,
        ui.visuals().weak_text_color(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouping() {
        assert_eq!(group(16384), "16,384");
        assert_eq!(group(1022), "1,022");
        assert_eq!(group(2), "2");
    }

    #[test]
    fn snapping() {
        assert_eq!(effective(22, &[24, 26, 27, 28, 29, 30]), 24);
        assert_eq!(effective(25, &[24, 26, 27]), 24);
        assert_eq!(effective(31, &[8, 10, 11, 30]), 30);
    }

    #[test]
    fn table_window() {
        assert_eq!(window(10, Some(3), 100), (0, 10));
        assert_eq!(window(1000, Some(990), 100), (900, 100));
        assert_eq!(window(1000, Some(500), 100), (450, 100));
    }

    #[test]
    fn history_dedup() {
        let mut h = vec!["a".to_string(), "b".to_string()];
        remember(&mut h, "b");
        assert_eq!(h, ["b", "a"]);
    }
}
