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
        <div class="max-w-6xl mx-auto">
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
