//! SVG icon components.

use dioxus::prelude::*;

/// Shield icon for the logo and status.
#[component]
pub fn ShieldIcon(class: Option<String>) -> Element {
    let class = class.unwrap_or_default();

    rsx! {
        svg {
            class: "{class}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path {
                d: "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"
            }
        }
    }
}

/// Check circle icon for success states.
#[component]
pub fn CheckCircleIcon(class: Option<String>) -> Element {
    let class = class.unwrap_or_default();

    rsx! {
        svg {
            class: "{class}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            path {
                d: "M22 11.08V12a10 10 0 1 1-5.93-9.14"
            }
            polyline {
                points: "22 4 12 14.01 9 11.01"
            }
        }
    }
}

/// X circle icon for blocked/error states.
#[component]
pub fn XCircleIcon(class: Option<String>) -> Element {
    let class = class.unwrap_or_default();

    rsx! {
        svg {
            class: "{class}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            circle {
                cx: "12",
                cy: "12",
                r: "10"
            }
            line {
                x1: "15",
                y1: "9",
                x2: "9",
                y2: "15"
            }
            line {
                x1: "9",
                y1: "9",
                x2: "15",
                y2: "15"
            }
        }
    }
}

/// Alert triangle icon for warnings.
#[component]
pub fn AlertTriangleIcon(class: Option<String>) -> Element {
    let class = class.unwrap_or_default();

    rsx! {
        svg {
            class: "{class}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            path {
                d: "M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
            }
            line {
                x1: "12",
                y1: "9",
                x2: "12",
                y2: "13"
            }
            line {
                x1: "12",
                y1: "17",
                x2: "12.01",
                y2: "17"
            }
        }
    }
}
