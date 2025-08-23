use leptos::{
    ev,
    html::{button, div, h1, h2, header, p},
    prelude::*,
    task::spawn_local,
};
use leptos_meta::{provide_meta_context, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment, WildcardSegment,
};

use crate::components::psf_generator::PsfGenerator;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/gmt-cfd-psf-web.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=move || "Not found.">
                    <Route path=StaticSegment("") view=home_page/>
                    <Route path=WildcardSegment("any") view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
// #[component]
fn home_page() -> impl IntoView {
    // Creates a reactive value to update the button
    let count = RwSignal::new(0);
    // let on_click = move |i: i32| *count.write() += 1;

    // view! {
    //     <h1>"Welcome to Leptos!"</h1>
    //     <button on:click=on_click>"Click Me: " {count}</button>
    // }
    div().child(
        header().child((
            h1().child("GMT CFD PSF Generator"),
            p().child("Generate Point Spread Function visualizations from GMT telescope CFD data"),
            config_form(),
            button()
                // .on(ev::click, move |_| *count.write() += 1)
                .on(ev::click, move |_| {
                    spawn_local(async {
                        cfd_psfs().await.expect("failed to generate CFD PSFs");
                    })
                })
                .child("CFD PSFs"),
        )),
    )
}

pub fn config_form() -> impl IntoView {
    div().child((h2().child("PSF Configuration")))
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

#[server]
pub async fn cfd_psfs() -> Result<(), ServerFnError> {
    println!("** CFD PSFS **");
    #[cfg(feature = "ssr")]
    {
        use gmt_cfd_psf::GmtOpticalModel;
        // Setup GMT optics and imaging
        let mut gmt = GmtOpticalModel::new()?;
        // Generate reference frame (no turbulence)
        gmt.ray_trace().read_detector().save("psf.png")?;
        println!("Saved frame0 as psf.png");
    }
    Ok(())
}
