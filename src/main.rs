use leptos::{*, html::Div};
use reqwasm::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};
use gloo_timers::callback::Interval;
use std::rc::Rc;

const BACKEND_IP: &str = "127.0.0.1";

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
struct Metrics { cpu_usec: u64, memory_bytes: u64 }

#[derive(Serialize, Deserialize)]
struct CreateRequest { 
    id: String, 
    hostname: String, 
    command: String, 
    args: Vec<String>,
    cores: u32,
    memory_mib: u64,
}

#[component]
fn Dashboard() -> impl IntoView {
    let (container_id, _) = create_signal("mc-server-01".to_string());

    let launch_minecraft = move |_| {
        logging::log!(">>> [UI] Botão START clicado. Iniciando requisição...");
        spawn_local(async move {
            let req = CreateRequest {
                id: "mc-server-01".to_string(),
                hostname: "survival".to_string(),
                command: "/usr/bin/java".to_string(),
                args: vec!["-Xmx2G".to_string(), "-jar".to_string(), "server.jar".to_string()],
                cores: 2,
                memory_mib: 2048,
            };

            let window = web_sys::window().unwrap();
            let hostname = window.location().hostname().unwrap_or_else(|_| "127.0.0.1".into());
            let url = format!("http://{}:3000/containers", hostname);
            logging::log!(">>> [UI] Enviando POST para {}", url);

            match Request::post(&url)
                .body(serde_json::to_string(&req).unwrap())
                .header("Content-Type", "application/json")
                .send()
                .await {
                    Ok(resp) => {
                        logging::log!("<<< [UI] Resposta recebida! Status: {}", resp.status());
                    },
                    Err(e) => {
                        logging::log!("!!! [UI] ERRO CRÍTICO na requisição: {:?}", e);
                    }
                }
        });
    };

    view! {
        <div class="min-h-screen bg-slate-950 text-slate-100 p-8 font-sans">
            <header class="flex justify-between items-center mb-10 border-b border-slate-800 pb-6">
                <h1 class="text-4xl font-black text-white">"AXION"</h1>
                <button on:click=launch_minecraft class="bg-sky-600 hover:bg-sky-500 px-8 py-3 rounded-xl font-bold transition-all">
                    "🚀 START SERVER"
                </button>
            </header>
            <div class="grid grid-cols-1 lg:grid-cols-4 gap-8">
                <aside class="lg:col-span-1"><MetricsPanel container_id=container_id /></aside>
                <section class="lg:col-span-3"><TerminalPanel container_id=container_id /></section>
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
                let url = format!("http://{}:3000/containers/{}/metrics", BACKEND_IP, id_clone);
                if let Ok(resp) = Request::get(&url).send().await {
                    if let Ok(m) = resp.json::<Metrics>().await { set_metrics.set(m); }
                }
            });
        });
        on_cleanup(move || drop(handle));
    });
    view! {
        <div class="bg-slate-900 p-6 rounded-3xl border border-slate-800 shadow-2xl">
            <h2 class="text-xs font-bold text-slate-500 mb-8 tracking-widest">"PERFORMANCE"</h2>
            <div class="space-y-10">
                <div>
                    <div class="flex justify-between mb-4"><span class="text-sm">"CPU"</span><span>{move || metrics.get().cpu_usec} " µs"</span></div>
                    <div class="w-full bg-slate-800 h-1 rounded-full overflow-hidden"><div class="bg-sky-500 h-full w-[10%]"></div></div>
                </div>
                <div>
                    <div class="flex justify-between mb-4"><span class="text-sm">"RAM"</span><span>{move || format!("{:.1} MB", metrics.get().memory_bytes as f64 / 1024.0 / 1024.0)}</span></div>
                    <div class="w-full bg-slate-800 h-1 rounded-full overflow-hidden"><div class="bg-emerald-500 h-full w-[5%]"></div></div>
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
            
            // Correção técnica: usamos a referência direta sem o casting que consome a posse
            let el: &web_sys::HtmlElement = &div;
            term.open(el);
            
            term.write("\x1b[1;34m[Axion]\x1b[0m Ready for command...\r\n");
            
            let ws_url = format!("ws://{}:3000/containers/{}/pty", BACKEND_IP, container_id.get());
            let ws = WebSocket::new(&ws_url).ok();
            if let Some(ref socket) = ws {
                let term_read = Rc::clone(&term);
                let on_msg = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Some(txt) = e.data().as_string() { term_read.write(&txt); }
                }) as Box<dyn FnMut(MessageEvent)>);
                socket.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
                on_msg.forget();
            }
            on_cleanup(move || { if let Some(s) = ws { let _ = s.close(); } });
        }
    });
    view! {
        <div class="bg-slate-900 rounded-3xl border border-slate-800 overflow-hidden shadow-2xl h-[600px]">
            <div class="bg-slate-800/30 px-6 py-4 border-b border-slate-800 flex justify-between">
                <span class="text-xs font-mono text-slate-500">"PTY_" {move || container_id.get()}</span>
            </div>
            <div node_ref=terminal_ref class="p-4 h-full"></div>
        </div>
    }
}

fn main() { mount_to_body(|| view! { <Dashboard /> }); }
