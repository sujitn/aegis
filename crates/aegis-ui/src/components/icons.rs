//! SVG icon components.

use dioxus::prelude::*;

/// Lock-open icon for the Aegis logo - matches the app icon.
#[component]
pub fn LockOpenIcon(class: Option<String>) -> Element {
    let class = class.unwrap_or_default();

    rsx! {
        svg {
            class: "{class}",
            view_box: "0 0 48 48",
            fill: "currentColor",
            // Lock body and open shackle
            path {
                d: "M40,18H16V13a7,7,0,0,1,7-7h2a7.1,7.1,0,0,1,5,2.1,2,2,0,0,0,2.2.5h.1a1.9,1.9,0,0,0,.6-3.1A10.9,10.9,0,0,0,25,2H23A11,11,0,0,0,12,13v5H8a2,2,0,0,0-2,2V44a2,2,0,0,0,2,2H40a2,2,0,0,0,2-2V20A2,2,0,0,0,40,18ZM38,42H10V22H38Z"
            }
            // Terminal prompt arrow
            path {
                d: "M15,40a2,2,0,0,1-1.3-3.5L19,32l-5.3-4.5a2,2,0,0,1,2.6-3l7,6a2,2,0,0,1,0,3l-7,6A1.9,1.9,0,0,1,15,40Z",
                opacity: "0.7"
            }
            // Terminal cursor
            path {
                d: "M33,38H27a2,2,0,0,1,0-4h6a2,2,0,0,1,0,4Z",
                opacity: "0.7"
            }
        }
    }
}

/// Alias for backward compatibility - use LockOpenIcon instead.
#[component]
pub fn ShieldIcon(class: Option<String>) -> Element {
    rsx! {
        LockOpenIcon { class: class }
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
