use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Meta, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment, WildcardSegment,
};

use crate::components::psf_generator::PsfGenerator;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/gmt-cfd-psf-web.css"/>
        <Stylesheet id="tailwind" href="https://cdn.tailwindcss.com"/>
        <Title text="GMT CFD PSF Generator"/>
        <Meta name="description" content="Generate Point Spread Function visualizations from GMT CFD data"/>

        <Router>
            <main class="container mx-auto p-4">
                <Routes fallback=move || "Not found.">
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=WildcardSegment("any") view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="max-w-6xl mx-auto relative">
            // Top-right corner icons
            <div class="absolute top-0 right-0 flex space-x-3 z-10">
                <a 
                    href="https://github.com/GMTO-Integrated-Modeling/gmt-cfd-psf"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="p-2 hover:bg-gray-100 rounded-full transition-colors duration-200"
                    title="View on GitHub"
                >
                    // GitHub icon SVG
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" class="text-gray-700 hover:text-black">
                        <path d="M12 0C5.374 0 0 5.373 0 12 0 17.302 3.438 21.8 8.207 23.387c.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.509 11.509 0 0112 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576C20.566 21.797 24 17.3 24 12c0-6.627-5.373-12-12-12z" fill="currentColor"/>
                    </svg>
                </a>
                
                <a 
                    href="https://www.gmto.org"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="p-2 hover:bg-gray-100 rounded-full transition-colors duration-200"
                    title="Visit GMTO"
                >
                    <img 
                        src="/assets/favicon.ico" 
                        alt="GMTO" 
                        width="24" 
                        height="24"
                        class="opacity-70 hover:opacity-100 transition-opacity duration-200"
                    />
                </a>
            </div>

            <header class="text-center mb-8">
                <h1 class="text-4xl font-bold text-blue-900 mb-2">
                    "GMT CFD PSF Generator"
                </h1>
                <p class="text-gray-600 text-lg">
                    "Generate Point Spread Function visualizations from GMT telescope CFD data"
                </p>
            </header>

            <PsfGenerator/>
        </div>
    }
}
/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        let resp = expect_context::<leptos_actix::ResponseOptions>();
        resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
    }

    view! {
        <h1>"Not Found"</h1>
    }
}
