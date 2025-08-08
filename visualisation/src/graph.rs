use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serde::Deserialize;
use stylist::css;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement};
use yew::{function_component, html, use_effect_with, use_memo, use_mut_ref, use_node_ref, use_state, Callback, Html, InputEvent, KeyboardEvent, MouseEvent, Properties, TargetCast, TouchEvent, UseStateHandle, WheelEvent};
use yew_icons::{Icon, IconId};

#[derive(Debug, Deserialize, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Node {
    pub key: String,
    pub attributes: NodeAttributes,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct NodeAttributes {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub size: f32,
    pub color: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Edge {
    pub key: String,
    pub source: String,
    pub target: String,
    pub attributes: EdgeAttributes,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct EdgeAttributes {
    pub weight: f64,
}

#[derive(Properties, PartialEq)]
pub struct CanvasGraphProps {
    pub width: u32,
    pub height: u32,
}

pub fn at_name_icon() -> Html {
    let logo_style = css!(r#"
        width: 3em;
        height: 3em;
        color: #fff;
        cursor: pointer;
    "#);
    html! {
        <svg class={logo_style.clone()} xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 52 52"><path fill-rule="evenodd" d="M34.632 5.055a20.968 20.968 0 1 0-16.034 36.75 25 25 0 0 0 4.128.13c3.431-.243 6.647-1.166 9.439-2.68a21 21 0 0 0 1.59-.956l12.368 12.369a3.226 3.226 0 0 0 4.565-4.562L38.464 33.882q.823-1.093 1.483-2.312h-3.324c-3.324 4.879-9.031 7.458-15.735 7.458-9.862 0-17.785-7.963-17.785-17.944 0-9.925 7.923-17.944 17.785-17.944 12.818 0 16.09 7.472 16.82 11.894.144.873.19 1.627.19 2.181 0 8.86-5.874 13.29-9.253 13.29-.942 0-1.663-.505-1.663-1.458 0-.785.444-2.131.72-2.916L33.41 9.028h-3.823l-.776 2.636c-1.164-2.468-3.712-3.757-6.316-3.757-8.256 0-13.852 9.588-13.852 17.158 0 4.598 2.992 8.468 7.757 8.468 2.382 0 4.931-1.066 6.649-2.86.665 2.075 2.88 2.972 4.82 2.972 2.168 0 5.647-1.023 8.48-3.8C38.946 27.3 41 23.277 41 17.215c0-.644-.11-2.339-.711-4.427a21 21 0 0 0-5.657-7.733M12.91 24.785c0-5.159 3.767-13.514 9.64-13.514 2.327 0 3.934 1.907 3.934 4.094 0 2.41-2.77 14.803-9.253 14.803-2.826 0-4.321-2.691-4.321-5.383" clip-rule="evenodd"/></svg>
    }
}

fn draw_edges_in_batches(context: &CanvasRenderingContext2d, graph: Rc<Graph>, drawn_edges: Rc<RefCell<usize>>, scale: f64, selected: UseStateHandle<Option<usize>>, raf_handle: Rc<RefCell<Option<i32>>>, timeout_handle: Rc<RefCell<Option<i32>>>,) {
    let context = context.clone();
    let drawn_edges = drawn_edges.clone();
    let selected_handle = selected.clone();
    let raf_handle = raf_handle.clone();
    let timeout_handle = timeout_handle.clone();

    let batch_size = 120;
    let total_edges = graph.edges.len();

    let closure: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    {
        let closure_inner = closure.clone();
        *closure_inner.borrow_mut() = Some(Closure::wrap(Box::new({
            let closure = closure.clone();
            let drawn_edges = drawn_edges.clone();
            let selected_handle = selected_handle.clone();
            let raf_handle = raf_handle.clone();
            let graph = graph.clone();
            let context = context.clone();
            move || {
                if selected_handle.is_some() {
                    return;
                }

                let mut start = drawn_edges.borrow_mut();
                let end = (*start + batch_size).min(total_edges);
                for edge in &graph.edges[*start..end] {
                    if selected_handle.is_some() {
                        return;
                    }
                    let s = graph.nodes.iter().find(|n| n.key == edge.source).unwrap();
                    let t = graph.nodes.iter().find(|n| n.key == edge.target).unwrap();
                    let sx = s.attributes.x / 10.0;
                    let sy = -s.attributes.y / 10.0;
                    let tx = t.attributes.x / 10.0;
                    let ty = -t.attributes.y / 10.0;

                    let col = s.attributes.color.replace('#', "");
                    if let Ok(rgb) = u32::from_str_radix(&col, 16) {
                        let r = (rgb >> 16) & 0xFF;
                        let g = (rgb >> 8) & 0xFF;
                        let b = rgb & 0xFF;
                        context.set_stroke_style_str(&format!("rgba({}, {}, {}, {})", r, g, b, 0.07));
                    }

                    let dx = tx - sx;
                    let dy = ty - sy;
                    let len = (dx*dx + dy*dy).sqrt();
                    let (nx, ny) = if len != 0.0 { (dy/len, -dx/len) } else { (0.0,0.0) };
                    let curve_offset = (len/5.0).min(40.0).max(10.0);
                    let cx = (sx+tx)/2.0 + nx * curve_offset;
                    let cy = (sy+ty)/2.0 + ny * curve_offset;

                    context.begin_path();
                    context.move_to(sx, sy);
                    context.quadratic_curve_to(cx, cy, tx, ty);
                    context.set_line_width(1.0 - (scale/11.0));
                    context.stroke();
                }
                *start = end;

                if end < total_edges {
                    let raf_id = web_sys::window()
                        .unwrap()
                        .request_animation_frame(
                            closure.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                        )
                        .unwrap();
                    *raf_handle.borrow_mut() = Some(raf_id);
                }
            }
        }) as Box<dyn FnMut()>));
    }

    let to_closure = closure.clone();
    let timeout_cb = Closure::once_into_js(move || {
        let win = web_sys::window().unwrap();
        win.request_animation_frame(
            to_closure
                .borrow()
                .as_ref()
                .unwrap()
                .as_ref()
                .unchecked_ref(),
        )
        .unwrap();
    });
    let win2 = web_sys::window().unwrap();
    let to_id = win2
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            timeout_cb.as_ref().unchecked_ref(),
            500,
        )
        .unwrap();
    *timeout_handle.borrow_mut() = Some(to_id);
    std::mem::forget(timeout_cb);
}

#[function_component(CanvasGraph)]
pub fn canvas_graph(props: &CanvasGraphProps) -> Html {
    let canvas_edges_ref = use_node_ref();
    let canvas_nodes_ref = use_node_ref();
    let width = props.width;
    let height = props.height;
    
    let scale = use_state(|| 0.8f64);
    let offset_x = use_state(|| -200.0f64);
    let offset_y = use_state(|| 0.0f64);

    let selected_node = use_state(|| None::<usize>);
    let search_open = use_state(|| false);
    let search_query = use_state(|| String::new());

    let scale_ref = use_mut_ref(|| *scale);
    let offset_x_ref = use_mut_ref(|| *offset_x);
    let offset_y_ref = use_mut_ref(|| *offset_y);

    let is_dragging = use_mut_ref(|| false);
    let last_mouse = use_mut_ref(|| (0.0f64, 0.0f64));
    
    let drawn_edges = use_mut_ref(|| 0usize);
    let raf_handle = use_mut_ref(|| None::<i32>);
    let timeout_handle = use_mut_ref(|| None::<i32>);
    

    {
        let drawn_edges = drawn_edges.clone();
        use_effect_with((scale.clone(), offset_x.clone(), offset_y.clone()), move |_| {
            *drawn_edges.borrow_mut() = 0;
            || ()
        });
    }

    let graph_rc = use_memo((), |_| {
        let raw: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/graph"));
        serde_json::from_str::<Graph>(raw)
            .expect("graph.json should parse to a Graph")
    });

    
    let graph = graph_rc.clone();

    let matches_ref = use_mut_ref(|| Vec::<(usize, String)>::new());
    let matches = {
        let query = (*search_query).to_lowercase().trim_matches('@').to_owned();
        if query.len() >= 3 {
            graph.nodes.iter().enumerate()
                .filter(|(_, node)| node.attributes.label.to_lowercase().contains(&query))
                .map(|(i, node)| (i, node.attributes.label.clone()))
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    };

    *matches_ref.borrow_mut() = matches.clone();

    let toggle_search = {
        let search_open = search_open.clone();
        Callback::from(move |_| search_open.set(!*search_open))
    };

    let oninput_search = {
        let search_query = search_query.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                search_query.set(input.value());
            }
        })
    };

    let on_select = {
        let scale = scale.clone();
        let scale_ref = scale_ref.clone();
        let offset_x = offset_x.clone();
        let offset_x_ref = offset_x_ref.clone();
        let offset_y = offset_y.clone();
        let offset_y_ref = offset_y_ref.clone();
        let selected_node = selected_node.clone();
        let search_open = search_open.clone();
        let graph = graph.clone();
        let search_query = search_query.clone();
        let last_mouse_ref = last_mouse.clone();

        Callback::from(move |idx: usize| {
            if let Some(node) = graph.nodes.get(idx) {
                let x = node.attributes.x/10.0;
                let y = -node.attributes.y/10.0;
                let s = 7.0;
                scale.set(s);
                *scale_ref.borrow_mut() = s;
                offset_x.set(-x * s);
                *offset_x_ref.borrow_mut() = -x * s;
                offset_y.set(-y * s);
                *offset_y_ref.borrow_mut() = -y * s;
                selected_node.set(Some(idx));
                *last_mouse_ref.borrow_mut() = (
                    (width  as f64) / 2.0,
                    (height as f64) / 2.0,
                );
            }
            search_open.set(false);
            search_query.set(String::new());
        })
    };

    let onkeydown_search = {
        let matches_ref = matches_ref.clone();
        let on_select = on_select.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let matches = matches_ref.borrow();
                if let Some((idx, _)) = matches.first() {
                    on_select.emit(*idx);
                }
            }
        })
    };

    let input_search_ref = use_node_ref();

    {
        let search_open = search_open.clone();
        let input_search_ref = input_search_ref.clone();
        use_effect_with((), move |_| {
            let window = window().unwrap();
            let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.ctrl_key() && event.key().to_lowercase() == "f" {
                    event.prevent_default();
                    search_open.set(true);
                    if let Some(input) = input_search_ref.cast::<HtmlInputElement>() {
                        input.focus().ok();
                    }
                }
            }) as Box<dyn FnMut(_)>);

            window
                .add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref())
                .unwrap();

            cb.forget();

            || {}
        });
    }

    {
        let canvas_edges_ref = canvas_edges_ref.clone();
        let graph = graph_rc.clone();
        let scale = *scale;
        let offset_x = *offset_x;
        let offset_y = *offset_y;
        let drawn_edges = drawn_edges.clone();
        let selected = selected_node.clone();

        use_effect_with((graph.clone(), width, height, scale, offset_x, offset_y, selected.clone()), move |_| {
            if let Some(canvas) = canvas_edges_ref.cast::<HtmlCanvasElement>() {
                let context = canvas
                    .get_context("2d").unwrap().unwrap()
                    .dyn_into::<CanvasRenderingContext2d>().unwrap();

                context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap();
                context.clear_rect(0.0, 0.0, width as f64, height as f64);

                context.translate((width as f64) / 2.0 + offset_x, (height as f64) / 2.0 + offset_y).unwrap();
                context.scale(scale, scale).unwrap();

                *drawn_edges.borrow_mut() = 0;
                draw_edges_in_batches(&context, graph, drawn_edges, scale.clone(), selected, raf_handle.clone(), timeout_handle.clone());
            }
            move || {
                if let Some(to) = timeout_handle.borrow_mut().take() {
                    web_sys::window().unwrap().clear_timeout_with_handle(to);
                }
                if let Some(r) = raf_handle.borrow_mut().take() {
                    web_sys::window().unwrap().cancel_animation_frame(r).ok();
                }
            }
        });
    }

    {
        let canvas_ref = canvas_nodes_ref.clone();
        let graph = graph_rc.clone();
        let scale = *scale;
        let offset_x = *offset_x;
        let offset_y = *offset_y;
        let selected = (*selected_node).clone();
        use_effect_with((graph.clone(), width, height, scale, offset_x, offset_y, selected.clone()), move |_| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let context = canvas
                    .get_context("2d").unwrap().unwrap()
                    .dyn_into::<CanvasRenderingContext2d>().unwrap();

                context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap();
                context.clear_rect(0.0, 0.0, width as f64, height as f64);

                context.translate((width as f64) / 2.0 + offset_x, (height as f64) / 2.0 + offset_y).unwrap();
                context.scale(scale, scale).unwrap();

                let margin = 100.0 / scale;
                let view_left = -(width as f64 / 2.0 + offset_x) / scale - margin;
                let view_right = (width as f64 / 2.0 - offset_x) / scale + margin;
                let view_top = -(height as f64 / 2.0 + offset_y) / scale - margin;
                let view_bottom = (height as f64 / 2.0 - offset_y) / scale + margin;

                if let Some(sel_index) = selected {
                    if let Some(sel_node) = graph.nodes.get(sel_index) {
                        let selected_key = &sel_node.key;
                        let mut connected: HashMap<String, f64> = HashMap::new();

                        for edge in &graph.edges {
                            if &edge.source == selected_key {
                                connected.insert(edge.target.clone(), edge.attributes.weight);
                            } else if &edge.target == selected_key {
                                connected.insert(edge.source.clone(), edge.attributes.weight);
                            }
                        }
                        connected.insert(selected_key.to_string(), 10.0);

                        for edge in &graph.edges {
                            if &edge.source == selected_key || &edge.target == selected_key {
                                let source = graph.nodes.iter().find(|n| n.key == edge.source).unwrap();
                                let target = graph.nodes.iter().find(|n| n.key == edge.target).unwrap();

                                let sx = source.attributes.x / 10.0;
                                let sy = -source.attributes.y / 10.0;
                                let tx = target.attributes.x / 10.0;
                                let ty = -target.attributes.y / 10.0;

                                let edge_color = source.attributes.color.replace('#', "");
                                if let Ok(rgb) = u32::from_str_radix(&edge_color, 16) {
                                    let r = (rgb >> 16) & 0xFF;
                                    let g = (rgb >> 8) & 0xFF;
                                    let b = rgb & 0xFF;
                                    context.set_stroke_style_str(&format!("rgba({}, {}, {}, {})", r, g, b, (edge.attributes.weight.log2() / 10.0)));
                                }

                                let dx = tx - sx;
                                let dy = ty - sy;
                                let dist = (dx*dx + dy*dy).sqrt();
                                let (nx, ny) = if dist != 0.0 { (dy/dist, -dx/dist) } else { (0.0, 0.0) };
                                let offset = (dist / 5.0).min(40.0).max(10.0);
                                let cx = (sx + tx)/2.0 + nx * offset;
                                let cy = (sy + ty)/2.0 + ny * offset;

                                context.begin_path();
                                context.move_to(sx, sy);
                                context.quadratic_curve_to(cx, cy, tx, ty);
                                context.set_line_width(edge.attributes.weight / 10.0);
                                context.stroke();
                            }
                        }

                        for node in &graph.nodes {
                            let x = node.attributes.x / 10.0;
                            let y = -node.attributes.y / 10.0;
                            let size_factor = (node.attributes.size as f64).log2() / 2.0;

                            if x + size_factor < view_left || x - size_factor > view_right ||
                            y + size_factor < view_top || y - size_factor > view_bottom {
                                continue;
                            }

                            context.begin_path();
                            context.arc(x, y, size_factor, 0.0, std::f64::consts::TAU).unwrap();
                            context.set_fill_style_str(&node.attributes.color);
                            context.fill();
                        }

                        for node in &graph.nodes {
                            if !connected.contains_key(&node.key) {
                                continue;
                            }

                            let x = node.attributes.x / 10.0;
                            let y = -node.attributes.y / 10.0;
                            let size_factor = (node.attributes.size as f64).log2() / 2.0;

                            context.set_fill_style_str("#ffffff");
                            let weight = *connected.get(&node.key).unwrap_or(&10.0);
                            context.set_font(
                                format!("bold {}px Univers", ((5.0 + weight) / scale).max(2.0)).as_str(),
                            );
                            context
                                .fill_text(
                                    &node.attributes.label,
                                    x + size_factor,
                                    y + size_factor / 2.0,
                                )
                                .unwrap();
                        }
                    }
                } else {
                    for node in &graph.nodes {
                        let x = node.attributes.x / 10.0;
                        let y = -node.attributes.y / 10.0;
                        let size_factor = (node.attributes.size as f64).log2() / 2.0;

                        if x + size_factor < view_left || x - size_factor > view_right ||
                        y + size_factor < view_top || y - size_factor > view_bottom {
                            continue;
                        }

                        context.begin_path();
                        context.arc(x, y, size_factor, 0.0, std::f64::consts::TAU).unwrap();
                        context.set_fill_style_str(&node.attributes.color);
                        context.fill();
                    }

                    for node in &graph.nodes {
                        let x = node.attributes.x / 10.0;
                        let y = -node.attributes.y / 10.0;
                        let size_factor = ((node.attributes.size as f64).log2() / 2.0).max(1.0);

                        if x + size_factor < view_left || x - size_factor > view_right ||
                        y + size_factor < view_top || y - size_factor > view_bottom 
                        || scale < (5.0/(size_factor*size_factor)) {
                            continue;
                        }

                        context.set_fill_style_str("#ffffff");
                        context.set_font(format!("bold {}px Univers", ((7.0 + (node.attributes.size as f64)) / scale).max(2.0)).as_str());
                        context
                            .fill_text(
                                &node.attributes.label,
                                x + size_factor,
                                y + size_factor / 2.0,
                            )
                            .unwrap();
                    }
                }
            }
            || ()
        });
    }

    {
        let canvas_ref = canvas_nodes_ref.clone();
        let scale = scale.clone();
        let offset_x = offset_x.clone();
        let offset_y = offset_y.clone();
        let scale_ref = scale_ref.clone();
        let offset_x_ref = offset_x_ref.clone();
        let offset_y_ref = offset_y_ref.clone();
        let is_dragging = is_dragging.clone();
        let last_mouse = last_mouse.clone();
        let sel_state = selected_node.clone();
        let did_move = Rc::new(RefCell::new(false));
        let initial_pinch_distance = Rc::new(RefCell::new(None::<f64>));
        let initial_scale = Rc::new(RefCell::new(1.0));
        let initial_offset_x = Rc::new(RefCell::new(0.0));
        let initial_offset_y = Rc::new(RefCell::new(0.0));

        use_effect_with((), move |_| {
            let canvas = canvas_ref
                .cast::<HtmlCanvasElement>()
                .expect("Failed to cast to HtmlCanvasElement");
            let canvas_cloned = canvas.clone();
            let canvas_cloned2 = canvas.clone();
            let did_move_down = did_move.clone();
            let did_move_move = did_move.clone();
            let did_move_click = did_move.clone();

            let is_dragging_mouse_down = is_dragging.clone();
            let last_mouse_down = last_mouse.clone();
            let on_mouse_down = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                *is_dragging_mouse_down.borrow_mut() = true;
                *did_move_down.borrow_mut() = false;
                *last_mouse_down.borrow_mut() = (e.client_x() as f64, e.client_y() as f64);
            }) as Box<dyn FnMut(_)>);

            let is_dragging_mouse_up = is_dragging.clone();
            let on_mouse_up = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
                *is_dragging_mouse_up.borrow_mut() = false;
            }) as Box<dyn FnMut(_)>);

            let is_dragging_mouse_move = is_dragging.clone();
            let last_mouse_move = last_mouse.clone();
            let offset_x_move = offset_x.clone();
            let offset_y_move = offset_y.clone();
            let offset_x_ref_move = offset_x_ref.clone();
            let offset_y_ref_move = offset_y_ref.clone();
            let on_mouse_move = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                if *is_dragging_mouse_move.borrow() {
                    *did_move_move.borrow_mut() = true;
                    let (lx, ly) = *last_mouse_move.borrow();
                    let (cx, cy) = (e.client_x() as f64, e.client_y() as f64);
                    let dx = cx - lx;
                    let dy = cy - ly;

                    *offset_x_ref_move.borrow_mut() += dx;
                    *offset_y_ref_move.borrow_mut() += dy;
                    offset_x_move.set(*offset_x_ref_move.borrow());
                    offset_y_move.set(*offset_y_ref_move.borrow());
                    *last_mouse_move.borrow_mut() = (cx, cy);
                }
            }) as Box<dyn FnMut(_)>);

            let offset_x_wheel = offset_x.clone();
            let offset_y_wheel = offset_y.clone();
            let offset_x_ref_wheel = offset_x_ref.clone();
            let offset_y_ref_wheel = offset_y_ref.clone();
            let scale_wheel = scale.clone();
            let scale_ref_wheel = scale_ref.clone();
            let canvas_wheel = canvas.clone();
            let on_wheel = Closure::wrap(Box::new(move |e: WheelEvent| {
                e.prevent_default();

                let rect = canvas_wheel.get_bounding_client_rect();
                let mouse_x = e.client_x() as f64 - rect.left();
                let mouse_y = e.client_y() as f64 - rect.top();

                let delta = -e.delta_y();
                let zoom_factor = 1.1;
                let old_scale = *scale_ref_wheel.borrow();
                let mut new_scale = if delta > 0.0 {
                    old_scale * zoom_factor
                } else {
                    old_scale / zoom_factor
                };
                let min_scale = 0.8;
                let max_scale = 10.0;
                new_scale = new_scale.clamp(min_scale, max_scale);

                let dx = mouse_x - (canvas_wheel.width() as f64 / 2.0 + *offset_x_ref_wheel.borrow());
                let dy = mouse_y - (canvas_wheel.height() as f64 / 2.0 + *offset_y_ref_wheel.borrow());

                *offset_x_ref_wheel.borrow_mut() -= dx * (new_scale / old_scale - 1.0);
                *offset_y_ref_wheel.borrow_mut() -= dy * (new_scale / old_scale - 1.0);
                *scale_ref_wheel.borrow_mut() = new_scale;

                offset_x_wheel.set(*offset_x_ref_wheel.borrow());
                offset_y_wheel.set(*offset_y_ref_wheel.borrow());
                scale_wheel.set(*scale_ref_wheel.borrow());
            }) as Box<dyn FnMut(_)>);

            let graph = graph_rc.clone();
            let sel_state_click = sel_state.clone();
            let canvas_on_click = canvas.clone();
            let on_click = Closure::wrap(Box::new(move |e: MouseEvent| {
                if *did_move_click.borrow() {return;}
                let rect = canvas_on_click.get_bounding_client_rect();
                let sx = e.client_x() as f64 - rect.left();
                let sy = e.client_y() as f64 - rect.top();

                let ctx = canvas_on_click
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<CanvasRenderingContext2d>()
                    .unwrap();

                let transform = ctx.get_transform().unwrap();
                let inv = transform.inverse();

                let graph_x = inv.a() * sx + inv.c() * sy + inv.e();
                let graph_y = inv.b() * sx + inv.d() * sy + inv.f();

                let mut found = None;
                for (idx, node) in graph.nodes.iter().enumerate() {
                    let nx = node.attributes.x / 10.0;
                    let ny = -node.attributes.y / 10.0;
                    let r  = (node.attributes.size as f64).log2() / 2.0;
                    let dx = graph_x - nx;
                    let dy = graph_y - ny;
                    if dx*dx + dy*dy <= r*r {
                        found = Some(idx);
                        break;
                    }
                }
                sel_state_click.set(found);
            }) as Box<dyn FnMut(_)>);

            let is_dragging_touch_start = is_dragging.clone();
            let last_mouse_touch_start = last_mouse.clone();
            let did_move_touch_start = did_move.clone();
            let initial_pinch_distance_start = initial_pinch_distance.clone();
            let initial_scale_start = initial_scale.clone();
            let scale_ref_start = scale_ref.clone();
            let initial_offset_x_start = initial_offset_x.clone();
            let initial_offset_y_start = initial_offset_y.clone();
            let offset_x_ref_start = offset_x_ref.clone();
            let offset_y_ref_start = offset_y_ref.clone();

            let on_touch_start = Closure::wrap(Box::new(move |e: TouchEvent| {
                e.prevent_default();

                if e.touches().length() == 2 {
                    let t1 = e.touches().get(0).unwrap();
                    let t2 = e.touches().get(1).unwrap();
                    let dx = t2.client_x() as f64 - t1.client_x() as f64;
                    let dy = t2.client_y() as f64 - t1.client_y() as f64;
                    *initial_pinch_distance_start.borrow_mut() = Some((dx * dx + dy * dy).sqrt());
                    *initial_scale_start.borrow_mut() = *scale_ref_start.borrow();
                    *initial_offset_x_start.borrow_mut() = *offset_x_ref_start.borrow();
                    *initial_offset_y_start.borrow_mut() = *offset_y_ref_start.borrow(); 
                } else {
                    *initial_pinch_distance_start.borrow_mut() = None;
                    *is_dragging_touch_start.borrow_mut() = true;
                    *did_move_touch_start.borrow_mut() = false;
                    if let Some(touch) = e.touches().get(0) {
                        *last_mouse_touch_start.borrow_mut() = (
                            touch.client_x() as f64,
                            touch.client_y() as f64,
                        );
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let is_dragging_touch_move = is_dragging.clone();
            let last_mouse_touch_move = last_mouse.clone();
            let offset_x_touch = offset_x.clone();
            let offset_y_touch = offset_y.clone();
            let offset_x_ref_touch = offset_x_ref.clone();
            let offset_y_ref_touch = offset_y_ref.clone();
            let did_move_touch_move = did_move.clone();
            let initial_pinch_distance_move = initial_pinch_distance.clone();
            let initial_scale_move = initial_scale.clone();
            let canvas_for_zoom = canvas_cloned.clone();
            let offset_x_zoom = offset_x.clone();
            let offset_y_zoom = offset_y.clone();
            let offset_x_ref_zoom = offset_x_ref.clone();
            let offset_y_ref_zoom = offset_y_ref.clone();
            let scale_zoom = scale.clone();
            let scale_ref_zoom = scale_ref.clone();

            let on_touch_move = Closure::wrap(Box::new(move |e: TouchEvent| {
                e.prevent_default();

                    if e.touches().length() == 2 {
                        let t1 = e.touches().get(0).unwrap();
                        let t2 = e.touches().get(1).unwrap();

                        let dx = t2.client_x() as f64 - t1.client_x() as f64;
                        let dy = t2.client_y() as f64 - t1.client_y() as f64;
                        let current_distance = (dx * dx + dy * dy).sqrt();

                        if let Some(initial_distance) = *initial_pinch_distance_move.borrow() {
                            let scale_factor = current_distance / initial_distance;
                            let old_scale = *initial_scale_move.borrow();
                            let new_scale = (old_scale * scale_factor).clamp(0.8, 10.0);

                            let mid_x = (t1.client_x() as f64 + t2.client_x() as f64) / 2.0;
                            let mid_y = (t1.client_y() as f64 + t2.client_y() as f64) / 2.0;

                            let rect = canvas_for_zoom.get_bounding_client_rect();

                            let canvas_mid_x_css = mid_x - rect.left();
                            let canvas_mid_y_css = mid_y - rect.top();

                            let dx_canvas = canvas_mid_x_css - (rect.width() as f64 / 2.0 + *initial_offset_x.borrow());
                            let dy_canvas = canvas_mid_y_css - (rect.height() as f64 / 2.0 + *initial_offset_y.borrow());

                            let scale_change = new_scale / old_scale;
                            let new_offset_x = *initial_offset_x.borrow() - dx_canvas * (scale_change - 1.0);
                            let new_offset_y = *initial_offset_y.borrow() - dy_canvas * (scale_change - 1.0);

                            *offset_x_ref_zoom.borrow_mut() = new_offset_x;
                            *offset_y_ref_zoom.borrow_mut() = new_offset_y;

                            *scale_ref_zoom.borrow_mut() = new_scale;
                            scale_zoom.set(new_scale);
                            offset_x_zoom.set(new_offset_x);
                            offset_y_zoom.set(new_offset_y);
                        }

                        return;
                    }
                if *is_dragging_touch_move.borrow() {
                    if let Some(t) = e.touches().get(0) {
                        *did_move_touch_move.borrow_mut() = true;
                        let (lx, ly) = *last_mouse_touch_move.borrow();
                        let (cx, cy) = (t.client_x() as f64, t.client_y() as f64);
                        let dx = cx - lx;
                        let dy = cy - ly;

                        *offset_x_ref_touch.borrow_mut() += dx;
                        *offset_y_ref_touch.borrow_mut() += dy;
                        offset_x_touch.set(*offset_x_ref_touch.borrow());
                        offset_y_touch.set(*offset_y_ref_touch.borrow());
                        *last_mouse_touch_move.borrow_mut() = (cx, cy);
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let is_dragging_touch_end = is_dragging.clone();
            let did_move_touch_end = did_move.clone();
            let sel_state_touch = sel_state.clone();
            let canvas_for_tap = canvas_cloned2.clone();
            let graph_for_tap = graph_rc.clone();
            let initial_pinch_distance_end = initial_pinch_distance.clone();
            let on_touch_end = Closure::wrap(Box::new(move |e: TouchEvent| {
                e.prevent_default();
                *is_dragging_touch_end.borrow_mut() = false;
                *initial_pinch_distance_end.borrow_mut() = None;
                if !*did_move_touch_end.borrow() {
                    if let Some(t) = e.changed_touches().get(0) {
                        let rect = canvas_for_tap.get_bounding_client_rect();
                        let sx = t.client_x() as f64 - rect.left();
                        let sy = t.client_y() as f64 - rect.top();

                        let ctx = canvas_for_tap
                            .get_context("2d")
                            .unwrap()
                            .unwrap()
                            .dyn_into::<CanvasRenderingContext2d>()
                            .unwrap();

                        let inv = ctx.get_transform().unwrap().inverse();
                        let graph_x = inv.a() * sx + inv.c() * sy + inv.e();
                        let graph_y = inv.b() * sx + inv.d() * sy + inv.f();

                        let mut found = None;
                        for (idx, node) in graph_for_tap.nodes.iter().enumerate() {
                            let nx = node.attributes.x / 10.0;
                            let ny = -node.attributes.y / 10.0;
                            let r  = (node.attributes.size as f64).log2() / 2.0;
                            let dx = graph_x - nx;
                            let dy = graph_y - ny;
                            if dx*dx + dy*dy <= r*r {
                                found = Some(idx);
                                break;
                            }
                        }
                        sel_state_touch.set(found);
                    }
                }
            }) as Box<dyn FnMut(_)>);

            canvas.add_event_listener_with_callback("mousedown", on_mouse_down.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("mouseup", on_mouse_up.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("mousemove", on_mouse_move.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref()).unwrap();

            canvas.add_event_listener_with_callback("touchstart", on_touch_start.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("touchmove",  on_touch_move.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("touchend",   on_touch_end.as_ref().unchecked_ref()).unwrap();


            on_mouse_down.forget();
            on_mouse_up.forget();
            on_mouse_move.forget();
            on_wheel.forget();
            on_click.forget();

            on_touch_start.forget();
            on_touch_move.forget();
            on_touch_end.forget();

            || ()
        });
    }

    let zoom_in = {
        let scale = scale.clone();
        let scale_ref = scale_ref.clone();
        let offset_x = offset_x.clone();
        let offset_y = offset_y.clone();
        let offset_x_ref = offset_x_ref.clone();
        let offset_y_ref = offset_y_ref.clone();
        Callback::from(move |_| {
            let old_scale = *scale_ref.borrow();
            let factor = 1.1;
            let new_scale = (old_scale * factor).clamp(0.8, 10.0);

            let cx = (width as f64) / 2.0;
            let cy = (height as f64) / 2.0;

            let dx = cx - ((width as f64)/2.0 + *offset_x_ref.borrow());
            let dy = cy - ((height as f64)/2.0 + *offset_y_ref.borrow());

            *offset_x_ref.borrow_mut() -= dx * (new_scale/old_scale - 1.0);
            *offset_y_ref.borrow_mut() -= dy * (new_scale/old_scale - 1.0);

            *scale_ref.borrow_mut() = new_scale;
            scale.set(new_scale);
            offset_x.set(*offset_x_ref.borrow());
            offset_y.set(*offset_y_ref.borrow());
        })
    };

    let zoom_out = {
        let scale = scale.clone();
        let scale_ref = scale_ref.clone();
        let offset_x = offset_x.clone();
        let offset_y = offset_y.clone();
        let offset_x_ref = offset_x_ref.clone();
        let offset_y_ref = offset_y_ref.clone();
        Callback::from(move |_| {
            let old_scale = *scale_ref.borrow();
            let factor = 1.1;
            let new_scale = (old_scale / factor).clamp(0.8, 10.0);

            let cx = (width as f64) / 2.0;
            let cy = (height as f64) / 2.0;

            let dx = cx - ((width as f64)/2.0 + *offset_x_ref.borrow());
            let dy = cy - ((height as f64)/2.0 + *offset_y_ref.borrow());

            *offset_x_ref.borrow_mut() -= dx * (new_scale/old_scale - 1.0);
            *offset_y_ref.borrow_mut() -= dy * (new_scale/old_scale - 1.0);

            *scale_ref.borrow_mut() = new_scale;
            scale.set(new_scale);
            offset_x.set(*offset_x_ref.borrow());
            offset_y.set(*offset_y_ref.borrow());
        })
    };

    let logo_style = css!(r#"
        width: 3em;
        height: 3em;
        color: #fff;
        cursor: pointer;
    "#);

    html! {
        <>
            <canvas
                    ref={canvas_edges_ref}
                    width={width.to_string()}
                    height={height.to_string()}
                    style="position: absolute; top: 0; left: 0; z-index: 0; user-select: none;"
                />
            <canvas
                    ref={canvas_nodes_ref}
                    width={width.to_string()}
                    height={height.to_string()}
                    style="position: absolute; top: 0; left: 0; z-index: 1; user-select: none;"
            />
            <div style="position: fixed; bottom: 1em; left:1em; display: flex; gap: 1em; z-index: 3;">
                <Icon icon_id={IconId::BootstrapZoomIn} class={logo_style.clone()} onclick={zoom_in.clone()} />
                <Icon icon_id={IconId::BootstrapZoomOut} class={logo_style.clone()} onclick={zoom_out.clone()}/>
                <div class={logo_style.clone()} onclick={toggle_search.clone()}>
                    {at_name_icon()}
                </div>
                <input
                    type="text"
                    placeholder="@username..."
                    value={(*search_query).clone()}
                    oninput={oninput_search}
                    onkeydown={onkeydown_search}
                    ref={input_search_ref}
                    autofocus=true
                    style={format!(
                        "width: 12em; margin: 0.5em 0 0.5em 0; opacity: {};",
                        if *search_open { "1" } else { "0" }
                    )}
                />
                if *search_open {
                    <ul style="max-height: 6em; overflow-y: auto; margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column-reverse; position: absolute; bottom: 3em; right: 0em; max-width: 12em; overflow-x: hidden;">
                        { for matches.iter().map(|(i, label)| {
                            let label_clone = label.clone();
                            let idx = *i;
                            let on_click = {
                                let on_select = on_select.clone();
                                Callback::from(move |_| on_select.emit(idx))
                            };
                            html! {
                                <li onclick={on_click}
                                    style="padding: 0.25em 0; cursor: pointer; color: white; width: 10.5em;"
                                >
                                    { label_clone }
                                </li>
                            }
                        }) }
                    </ul>
                }
            </div>
        </>
    }
}
