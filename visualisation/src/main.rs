use wasm_bindgen::{prelude::Closure, JsCast};
use yew::{function_component, html, use_effect_with, use_state, Html};

use crate::graph::CanvasGraph;

mod graph;
#[function_component(App)]
fn app() -> Html {
    let window_width = use_state(|| 950.0);
    let window_height = use_state(|| 950.0);
    let canvas_size = use_state(|| 950u32);

    {
        let window_width = window_width.clone();
        let window_height = window_height.clone();
        let canvas_size = canvas_size.clone();

        use_effect_with((), move |_| {
            let win = web_sys::window().expect("no global `window` exists");
            let win2 = win.clone();

            let set_size = move || {
                let w = win2
                    .inner_width()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(950.0);
                let h = win2
                    .inner_height()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(950.0);

                window_width.set(w);
                window_height.set(h);
                canvas_size.set((w.min(h)) as u32);
            };
            set_size();
            let resize_closure =
                Closure::wrap(Box::new(move |_ev: web_sys::Event| set_size()) as Box<dyn FnMut(_)>);
            win.add_event_listener_with_callback(
                "resize",
                resize_closure.as_ref().unchecked_ref(),
            )
            .expect("failed to register resize listener");
            resize_closure.forget();

            || ()
        });
    }

    html! {
        <div style="width:100vw; height:100vh; margin:0; padding:0; overflow: hidden;">
            <CanvasGraph width={*window_width as u32} height={*window_height as u32} />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
