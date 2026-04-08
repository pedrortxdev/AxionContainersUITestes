use leptos::{*, html::Div};
use reqwasm::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};
use gloo_timers::callback::Interval;
use std::rc::Rc;

// --- Bindings para Xterm.js ---
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    pub type Terminal;

    #[wasm_bindgen(constructor, js_namespace = window, js_class = "Terminal")]
    fn new(options: &JsValue) -> Terminal;

    #[wasm_bindgen(method, js_name = open)]
    fn open(this: &Terminal, parent: &web_sys::HtmlElement);

    #[wasm_bindgen(method, js_name = write)]
    fn write(this: &Terminal, data: &str);

    #[wasm_bindgen(method, js_name = onData)]
    fn on_data(this: &Terminal, callback: &js_sys::Function) -> JsValue;

    #[wasm_bindgen(method, js_name = dispose)]
    fn dispose(this: &Terminal);
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Metrics {
    cpu_usec: u64,
    memory_bytes: u64,
}

#[derive(Serialize, Deserialize)]
struct CreateRequest {
    id: String,
    hostname: String,
    command: String,
    args: Vec<String>,
}

#[component]
fn Dashboard() -> impl IntoView {
    let (container_id, _) = create_signal("mc-server-01".to_string());

    let launch_minecraft = move |_| {
        spawn_local(async move {
            let req = CreateRequest {
                id: "mc-server-01".to_string(),
                hostname: "survival".to_string(),
                command: "/usr/bin/java".to_string(),
                args: vec!["-Xmx2G".to_string(), "-jar".to_string(), "server.jar".to_string()],
            };

            let _ = Request::post("http://localhost:3000/containers")
                .body(serde_json::to_string(&req).unwrap())
                .header("Content-Type", "application/json")
                .send()
                .await;
        });
    };

    view! {
        <div class="min-h-screen bg-slate-950 text-slate-100 p-8 font-sans selection:bg-sky-500/30">
            <header class="flex justify-between items-center mb-10 border-b border-slate-800 pb-6">
                <div>
                    <h1 class="text-4xl font-black tracking-tight text-white flex items-center gap-2">
                        <span class="bg-sky-600 w-8 h-8 rounded-lg"></span>
                        "AXION" <span class="text-sky-500 text-2xl font-light">"Beta"</span>
                    </h1>
                    <p class="text-slate-500 text-xs mt-2 uppercase tracking-widest font-bold">"Engine Controller"</p>
                </div>
                <div class="flex gap-4">
                    <button 
                        on:click=launch_minecraft
                        class="bg-sky-600 hover:bg-sky-500 active:scale-95 px-8 py-3 rounded-xl font-bold transition-all shadow-[0_0_20px_rgba(14,165,233,0.3)] border border-sky-400/20"
                    >
                        "🚀 START SERVER"
                    </button>
                </div>
            </header>

            <div class="grid grid-cols-1 lg:grid-cols-4 gap-8">
                <aside class="lg:col-span-1">
                    <MetricsPanel container_id=container_id />
                </aside>
                
                <section class="lg:col-span-3">
                    <TerminalPanel container_id=container_id />
                </section>
            </div>
        </div>
    }
}

#[component]
fn MetricsPanel(container_id: ReadSignal<String>) -> impl IntoView {
    let (metrics, set_metrics) = create_signal(Metrics::default());

    create_effect(move |_| {
        let id = container_id.get();
        let handle = Interval::new(1000, move || {
            let id_clone = id.clone();
            spawn_local(async move {
                let url = format!("http://localhost:3000/containers/{}/metrics", id_clone);
                if let Ok(resp) = Request::get(&url).send().await {
                    if let Ok(m) = resp.json::<Metrics>().await {
                        set_metrics.set(m);
                    }
                }
            });
        });

        on_cleanup(move || {
            drop(handle);
        });
    });

    view! {
        <div class="bg-slate-900/50 p-6 rounded-3xl border border-slate-800 backdrop-blur-xl shadow-2xl">
            <h2 class="text-[10px] font-black uppercase tracking-[0.2em] text-slate-500 mb-8 flex items-center gap-2">
                <span class="w-2 h-2 rounded-full bg-sky-500"></span>
                "Live Analytics"
            </h2>
            
            <div class="space-y-10">
                <div class="group">
                    <div class="flex justify-between items-end mb-4">
                        <span class="text-sm font-semibold text-slate-400">"CPU Delta"</span>
                        <span class="text-lg font-mono text-white tracking-tighter">{move || format!("{:0>7}", metrics.get().cpu_usec)} " µs"</span>
                    </div>
                    <div class="w-full bg-slate-800 h-1 rounded-full overflow-hidden">
                        <div class="bg-sky-500 h-full w-[35%] shadow-[0_0_15px_rgba(14,165,233,0.6)] transition-all"></div>
                    </div>
                </div>

                <div>
                    <div class="flex justify-between items-end mb-4">
                        <span class="text-sm font-semibold text-slate-400">"Allocated RAM"</span>
                        <span class="text-lg font-mono text-emerald-400 tracking-tighter">
                            {move || format!("{:.1} MB", metrics.get().memory_bytes as f64 / 1024.0 / 1024.0)}
                        </span>
                    </div>
                    <div class="w-full bg-slate-800 h-1 rounded-full overflow-hidden">
                        <div class="bg-emerald-500 h-full w-[22%] shadow-[0_0_15px_rgba(16,185,129,0.6)] transition-all"></div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn TerminalPanel(container_id: ReadSignal<String>) -> impl IntoView {
    let terminal_ref = create_node_ref::<Div>();

    create_effect(move |_| {
        if let Some(div) = terminal_ref.get() {
            let opts = serde_wasm_bindgen::to_value(&serde_json::json!({
                "theme": { "background": "#020617", "foreground": "#cbd5e1", "cursor": "#0ea5e9" }
            })).unwrap();
            
            let term = Rc::new(Terminal::new(&opts));
            
            // Correção sênior de casting: usamos a referência direta do elemento web_sys
            let el: &web_sys::HtmlElement = &div;
            term.open(el);
            
            term.write("\x1b[1;34m[Axion]\x1b[0m Bridge active.\r\n");

            let ws_url = format!("ws://localhost:3000/containers/{}/pty", container_id.get());
            let ws = WebSocket::new(&ws_url).expect("WebSocket error");
            
            let term_read = Rc::clone(&term);
            let on_msg = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Some(txt) = e.data().as_string() {
                    term_read.write(&txt);
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            ws.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));

            let ws_write = ws.clone();
            let on_data = Closure::wrap(Box::new(move |data: String| {
                let _ = ws_write.send_with_str(&data);
            }) as Box<dyn FnMut(String)>);
            let data_disposable = term.on_data(on_data.as_ref().unchecked_ref());

            let ws_cleanup = ws.clone();
            let term_cleanup = Rc::clone(&term);
            on_cleanup(move || {
                let _ = ws_cleanup.close();
                term_cleanup.dispose();
                drop(on_msg);
                drop(on_data);
                drop(data_disposable);
            });
        }
    });

    view! {
        <div class="bg-slate-900 rounded-3xl border border-slate-800 overflow-hidden shadow-2xl ring-1 ring-white/5">
            <div class="bg-slate-800/30 px-6 py-4 border-b border-slate-800 flex justify-between items-center">
                <div class="flex items-center gap-4">
                    <div class="flex gap-2">
                        <div class="w-3 h-3 rounded-full bg-slate-700"></div>
                        <div class="w-3 h-3 rounded-full bg-slate-700"></div>
                    </div>
                    <span class="text-[10px] font-mono font-bold text-slate-500 uppercase tracking-widest">"container_" {move || container_id.get()}</span>
                </div>
                <div class="flex items-center gap-3 bg-emerald-500/10 px-3 py-1 rounded-full border border-emerald-500/20">
                    <div class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></div>
                    <span class="text-[9px] font-black text-emerald-500 uppercase tracking-widest">"Active Bridge"</span>
                </div>
            </div>
            <div node_ref=terminal_ref class="h-[600px] p-4"></div>
        </div>
    }
}

fn main() {
    mount_to_body(|| view! { <Dashboard /> });
}
