use std::{cell::RefCell, collections::HashSet, rc::Rc};

use serde::Deserialize;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::{function_component, html, use_effect_with, use_memo, use_mut_ref, use_node_ref, use_state, Html, MouseEvent, Properties, UseStateHandle, WheelEvent};

#[derive(Debug, Deserialize, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Node {
    pub key: String, // String -> u32
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
    pub key: String, // String -> u32
    pub source: String, // String -> u32
    pub target: String, // String -> u32
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

fn draw_edges_in_batches(context: &CanvasRenderingContext2d, graph: Rc<Graph>, drawn_edges: Rc<RefCell<usize>>, scale: f64, selected: UseStateHandle<Option<usize>>, raf_handle: Rc<RefCell<Option<i32>>>, timeout_handle: Rc<RefCell<Option<i32>>>,) {
    let context = context.clone();
    let drawn_edges = drawn_edges.clone();
    let selected_handle = selected.clone();
    let raf_handle = raf_handle.clone();
    let timeout_handle = timeout_handle.clone();

    let batch_size = 40;
    let total_edges = graph.edges.len();

    let closure: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    {
        let closure_inner = closure.clone();
        *closure_inner.borrow_mut() = Some(Closure::wrap(Box::new({
            let closure = closure.clone(); // ✅ clone here
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
                        context.set_stroke_style_str(&format!("rgba({}, {}, {}, {})", r, g, b, 0.1));
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
    
    let scale = use_state(|| 0.6f64);
    let offset_x = use_state(|| 0.0f64);
    let offset_y = use_state(|| 0.0f64);

    let selected_node = use_state(|| None::<usize>);

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
        let raw = include_str!("graph.json");
        serde_json::from_str::<Graph>(raw)
            .expect("graph.json should parse to a Graph")
    });

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
                        let mut connected: HashSet<String> = HashSet::new();
                        connected.insert(selected_key.clone());

                        for edge in &graph.edges {
                            if &edge.source == selected_key {
                                connected.insert(edge.target.clone());
                            } else if &edge.target == selected_key {
                                connected.insert(edge.source.clone());
                            }
                        }
                        connected.insert(selected_key.clone());

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
                            if !connected.contains(&node.key) {
                                continue;
                            }

                            let x = node.attributes.x / 10.0;
                            let y = -node.attributes.y / 10.0;
                            let size_factor = (node.attributes.size as f64).log2() / 2.0;

                            context.set_fill_style_str("#ffffff");
                            context.set_font(
                                format!("bold {}px Univers", (5.0 / scale).max(5.0)).as_str(),
                            );
                            context
                                .fill_text(
                                    &node.attributes.label
                                        .strip_prefix("@")
                                        .unwrap_or(&node.attributes.label),
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
                        let size_factor = (node.attributes.size as f64).log2() / 2.0;

                        if x + size_factor < view_left || x - size_factor > view_right ||
                        y + size_factor < view_top || y - size_factor > view_bottom 
                        || scale < (5.0/(size_factor*size_factor)) {
                            continue;
                        }

                        context.set_fill_style_str("#ffffff");
                        context.set_font(format!("bold {}px Univers", (10.0 / scale).max(3.0)).as_str());
                        context
                            .fill_text(
                                &node.attributes.label.strip_prefix("@").unwrap(),
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

        use_effect_with((), move |_| {
            let canvas = canvas_ref
                .cast::<HtmlCanvasElement>()
                .expect("Failed to cast to HtmlCanvasElement");
            let canvas_cloned = canvas.clone();
            let canvas_cloned2 = canvas.clone();

            let is_dragging_mouse_down = is_dragging.clone();
            let last_mouse_down = last_mouse.clone();
            let on_mouse_down = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                *is_dragging_mouse_down.borrow_mut() = true;
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

            let on_wheel = Closure::wrap(Box::new(move |e: WheelEvent| {
                e.prevent_default();

                let rect = canvas_cloned.get_bounding_client_rect();
                let mouse_x = e.client_x() as f64 - rect.left();
                let mouse_y = e.client_y() as f64 - rect.top();

                let delta = -e.delta_y();
                let zoom_factor = 1.1;
                let old_scale = *scale_ref.borrow();
                let mut new_scale = if delta > 0.0 {
                    old_scale * zoom_factor
                } else {
                    old_scale / zoom_factor
                };
                let min_scale = 0.6;
                let max_scale = 10.0;
                new_scale = new_scale.clamp(min_scale, max_scale);

                let dx = mouse_x - (canvas_cloned.width() as f64 / 2.0 + *offset_x_ref.borrow());
                let dy = mouse_y - (canvas_cloned.height() as f64 / 2.0 + *offset_y_ref.borrow());

                *offset_x_ref.borrow_mut() -= dx * (new_scale / old_scale - 1.0);
                *offset_y_ref.borrow_mut() -= dy * (new_scale / old_scale - 1.0);
                *scale_ref.borrow_mut() = new_scale;

                offset_x.set(*offset_x_ref.borrow());
                offset_y.set(*offset_y_ref.borrow());
                scale.set(*scale_ref.borrow());
            }) as Box<dyn FnMut(_)>);

            let graph = graph_rc.clone();
            let on_click = Closure::wrap(Box::new(move |e: MouseEvent| {
                let rect = canvas_cloned2.get_bounding_client_rect();
                let sx = e.client_x() as f64 - rect.left();
                let sy = e.client_y() as f64 - rect.top();

                let ctx = canvas_cloned2
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
                sel_state.set(found);
            }) as Box<dyn FnMut(_)>);

            canvas.add_event_listener_with_callback("mousedown", on_mouse_down.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("mouseup", on_mouse_up.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("mousemove", on_mouse_move.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref()).unwrap();
            canvas.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref()).unwrap();

            on_mouse_down.forget();
            on_mouse_up.forget();
            on_mouse_move.forget();
            on_wheel.forget();
            on_click.forget();

            || ()
        });
    }

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
        </>
    }
}
