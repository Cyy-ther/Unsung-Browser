use std::sync::{Arc, Mutex};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    dpi::{PhysicalPosition, PhysicalSize},
};
use wry::WebViewBuilder;
use serde::{Deserialize, Serialize};
use reqwest::blocking::Client;
use scraper::{Html, Selector};

#[derive(Serialize)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn fetch_search_results(query: &str) -> Vec<SearchResult> {
    let search_url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .unwrap();

    match client.get(&search_url).send() {
        Ok(response) => {
            if let Ok(body) = response.text() {
                parse_results(&body)
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

fn parse_results(html: &str) -> Vec<SearchResult> {
    let document = Html::parse_document(html);
    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__a").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();

    for element in document.select(&result_selector).take(15) {
        if let Some(title_elem) = element.select(&title_selector).next() {
            let title = title_elem.text().collect::<String>();
            let url = title_elem.value().attr("href").unwrap_or("").to_string();

            let snippet = element
                .select(&snippet_selector)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResult {
                    title: title.trim().to_string(),
                    url: if url.starts_with("//") {
                        format!("https:{}", url)
                    } else {
                        url
                    },
                    snippet: snippet.trim().to_string(),
                });
            }
        }
    }

    results
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cypher Browser")
        .with_inner_size(tao::dpi::LogicalSize::new(1920, 1080))
        .build(&event_loop)
        .unwrap();

    let html_content = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Cypher Browser</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: #0a0a0a;
            color: #fff;
            height: 100vh;
            display: flex;
            flex-direction: column;
        }
        .top-bar {
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            padding: 15px 20px;
            display: flex;
            align-items: center;
            gap: 15px;
            border-bottom: 2px solid #0f3460;
            box-shadow: 0 4px 20px rgba(0,0,0,0.5);
        }
        .logo {
            font-size: 24px;
            font-weight: 800;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin-right: 10px;
        }
        .nav-buttons { display: flex; gap: 8px; }
        .nav-btn {
            background: rgba(255,255,255,0.1);
            border: 1px solid rgba(255,255,255,0.2);
            color: white;
            width: 36px;
            height: 36px;
            border-radius: 8px;
            cursor: pointer;
            font-size: 18px;
            display: flex;
            align-items: center;
            justify-content: center;
            transition: all 0.2s;
        }
        .nav-btn:hover { background: rgba(255,255,255,0.2); transform: translateY(-2px); }
        .url-bar { flex: 1; display: flex; gap: 10px; }
        #urlInput {
            flex: 1;
            background: rgba(255,255,255,0.1);
            border: 1px solid rgba(255,255,255,0.2);
            padding: 10px 20px;
            border-radius: 25px;
            color: white;
            font-size: 14px;
            outline: none;
            transition: all 0.3s;
        }
        #urlInput:focus {
            background: rgba(255,255,255,0.15);
            border-color: #667eea;
            box-shadow: 0 0 20px rgba(102,126,234,0.3);
        }
        #urlInput::placeholder { color: rgba(255,255,255,0.5); }
        .go-btn {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border: none;
            color: white;
            padding: 10px 30px;
            border-radius: 25px;
            cursor: pointer;
            font-weight: 600;
            font-size: 14px;
            transition: all 0.3s;
        }
        .go-btn:hover { transform: translateY(-2px); box-shadow: 0 5px 20px rgba(102,126,234,0.4); }
        .content {
            flex: 1;
            overflow-y: auto;
            padding: 30px;
            background: linear-gradient(to bottom, #0a0a0a 0%, #1a1a2e 100%);
        }
        .home-screen { max-width: 1000px; margin: 0 auto; text-align: center; }
        .hero { margin-bottom: 60px; padding: 40px 0; }
        .hero h1 {
            font-size: 72px;
            font-weight: 900;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin-bottom: 20px;
        }
        .hero p { font-size: 22px; color: rgba(255,255,255,0.7); font-weight: 300; }
        .search-container {
            background: rgba(255,255,255,0.05);
            border: 2px solid rgba(102,126,234,0.3);
            border-radius: 50px;
            padding: 10px;
            display: flex;
            max-width: 700px;
            margin: 0 auto 60px auto;
            box-shadow: 0 10px 40px rgba(0,0,0,0.3);
        }
        #searchInput {
            flex: 1;
            background: transparent;
            border: none;
            padding: 18px 30px;
            color: white;
            font-size: 18px;
            outline: none;}
        #searchInput::placeholder { color: rgba(255,255,255,0.4); }
        .search-btn {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border: none;
            color: white;
            padding: 18px 40px;
            border-radius: 50px;
            cursor: pointer;
            font-weight: 700;
            font-size: 16px;
            transition: all 0.3s;
        }
        .search-btn:hover { transform: scale(1.05); box-shadow: 0 5px 30px rgba(102,126,234,0.5); }
        .loading { text-align: center; padding: 60px 20px; }
        .spinner {
            width: 60px;
            height: 60px;
            border: 4px solid rgba(102,126,234,0.2);
            border-top-color: #667eea;
            border-radius: 50%;
            animation: spin 1s linear infinite;
            margin: 0 auto 20px auto;
        }
        @keyframes spin { to { transform: rotate(360deg); } }
        .results { max-width: 900px; margin: 0 auto; }
        .result-item {
            background: rgba(255,255,255,0.05);
            border: 1px solid rgba(255,255,255,0.1);
            border-radius: 15px;
            padding: 25px;
            margin-bottom: 20px;
            transition: all 0.3s;
            cursor: pointer;
        }
        .result-item:hover {
            background: rgba(255,255,255,0.08);
            border-color: #667eea;
            transform: translateY(-5px);
            box-shadow: 0 10px 40px rgba(102,126,234,0.2);
        }
        .result-item h3 { color: #667eea; font-size: 22px; margin-bottom: 10px; font-weight: 600; }
        .result-item .url { color: #10b981; font-size: 14px; margin-bottom: 12px; word-break: break-all; }
        .result-item .snippet { color: rgba(255,255,255,0.7); line-height: 1.6; font-size: 15px; }
        .quick-links {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            max-width: 900px;
            margin: 0 auto;
        }
        .quick-link {
            background: rgba(255,255,255,0.05);
            border: 1px solid rgba(255,255,255,0.1);
            border-radius: 12px;
            padding: 20px;
            text-align: center;
            cursor: pointer;
            transition: all 0.3s;
        }
        .quick-link:hover { background: rgba(255,255,255,0.08); border-color: #667eea; transform: translateY(-5px); }
        .quick-link .icon { font-size: 32px; margin-bottom: 10px; }
        .quick-link .name { color: white; font-weight: 600; }
        .iframe-container { width: 100%; height: calc(100vh - 80px); background: white; }
        .iframe-container iframe { width: 100%; height: 100%; border: none; }
    </style>
</head>
<body>
    <div class="top-bar">
        <div class="logo">🔮 CYPHER</div>
        <div class="nav-buttons">
            <button class="nav-btn" onclick="goBack()">◄</button>
            <button class="nav-btn" onclick="goForward()">►</button>
            <button class="nav-btn" onclick="reload()">↻</button>
            <button class="nav-btn" onclick="goHome()">🏠</button>
        </div>
        <div class="url-bar">
            <input type="text" id="urlInput" placeholder="Search or enter URL..." />
            <button class="go-btn" onclick="navigate()">Go</button>
        </div>
    </div>
    <div class="content" id="content">
        <div class="home-screen">
            <div class="hero">
                <h1>CYPHER</h1>
                <p>Your Gateway to the Internet</p>
            </div>
            <div class="search-container">
                <input type="text" id="searchInput" placeholder="What are you looking for?" />
                <button class="search-btn" onclick="performSearch()">Search</button>
            </div>
            <div class="quick-links">
                <div class="quick-link" onclick="navigateTo('https://github.com')">
                    <div class="icon">💻</div><div class="name">GitHub</div>
                </div>
                <div class="quick-link" onclick="navigateTo('https://youtube.com')">
                    <div class="icon">📺</div><div class="name">YouTube</div>
                </div>
                <div class="quick-link" onclick="navigateTo('https://reddit.com')">
                    <div class="icon">🗨️</div><div class="name">Reddit</div>
                </div>
                <div class="quick-link" onclick="navigateTo('https://twitter.com')">
                    <div class="icon">🐦</div><div class="name">Twitter</div>
                </div>
            </div>
        </div></div>
    <script>
        const history = [];
        let historyIndex = -1;
        function addToHistory(url) {
            if (historyIndex < history.length - 1) history.splice(historyIndex + 1);
            history.push(url);
            historyIndex++;
        }
        function goBack() {
            if (historyIndex > 0) { historyIndex--; loadUrl(history[historyIndex], false); }
        }
        function goForward() {
            if (historyIndex < history.length - 1) { historyIndex++; loadUrl(history[historyIndex], false); }
        }
        function reload() { if (historyIndex >= 0) loadUrl(history[historyIndex], false); }
        function goHome() { location.reload(); }
        function navigate() {
            const input = document.getElementById('urlInput').value.trim();
            if (input) {
                let url = input;
                if (!url.startsWith('http://') && !url.startsWith('https://')) {
                    url = url.includes('.') && !url.includes(' ') ? 'https://' + url : 'https://duckduckgo.com/?q=' + encodeURIComponent(url);
                }
                loadUrl(url);
            }
        }
        function navigateTo(url) { document.getElementById('urlInput').value = url; loadUrl(url); }
        function loadUrl(url, addHistory = true) {
            if (addHistory) addToHistory(url);
            document.getElementById('urlInput').value = url;
            document.getElementById('content').innerHTML = `<div class="iframe-container"><iframe src="${url}" sandbox="allow-same-origin allow-scripts allow-popups allow-forms"></iframe></div>`;
        }
        function displayResults(results, query) {
            if (!results || results.length === 0) {
                document.getElementById('content').innerHTML = `
                    <div class="loading">
                        <div style="font-size: 80px; margin-bottom: 20px;">😔</div>
                        <h2>No results found for "${query}"</h2>
                        <p style="color: rgba(255,255,255,0.5); margin-top: 10px;">Try different keywords</p>
                    </div>
                `;
                return;
            }
            const html = `
                <div class="results"><h2 style="color: rgba(255,255,255,0.9); margin-bottom: 30px; font-size: 28px;">
                        Search Results for "${query}"
                    </h2>
                    ${results.map(r => `
                        <div class="result-item" onclick="navigateTo('${r.url.replace(/'/g, "\\'")}')">
                            <h3>${r.title}</h3>
                            <div class="url">${r.url}</div>
                            <div class="snippet">${r.snippet}</div>
                        </div>
                    `).join('')}
                </div>
            `;
            document.getElementById('content').innerHTML = html;
        }
        async function performSearch() {
            const query = document.getElementById('searchInput').value.trim();
            if (query) {
                document.getElementById('urlInput').value = `Search: ${query}`;
                document.getElementById('content').innerHTML = `<div class="loading"><div class="spinner"></div><h2>Searching...</h2></div>`;
                window.ipc.postMessage(JSON.stringify({ query: query }));
            }
        }
        document.getElementById('urlInput').addEventListener('keypress', (e) => { if (e.key === 'Enter') navigate(); });
        document.getElementById('searchInput').addEventListener('keypress', (e) => { if (e.key === 'Enter') performSearch(); });
    </script>
</body>
</html>"#;

    let webview: Arc<Mutex<Option<wry::WebView>>> = Arc::new(Mutex::new(None));
    let webview_clone = webview.clone();

    let size = window.inner_size();

    let wv = WebViewBuilder::new()
        .with_html(html_content)
        .with_bounds(wry::Rect {
            position: PhysicalPosition::new(0, 0),
            size: PhysicalSize::new(size.width, size.height),
        })
        .with_ipc_handler(move |request| {
            #[derive(Deserialize)]
            struct SearchRequest { query: String }

            if let Ok(req) = serde_json::from_str::<String>(&request.body()) {
                if let Ok(search_req) = serde_json::from_str::<SearchRequest>(&req) {
                    let results = fetch_search_results(&search_req.query);
                    if let Ok(json) = serde_json::to_string(&results) {
                        let script = format!(
                            "displayResults({}, '{}');",
                            json,
                            search_req.query.replace("'", "\\'")
                        );
                        if let Some(wv) = webview_clone.lock().unwrap().as_ref() {
                            let _ = wv.evaluate_script(&script);
                        }
                    }
                }
            }
        })
        .build_as_child(&window)
        .unwrap();

    *webview.lock().unwrap() = Some(wv);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
}
