use gloo_net::http::Request;
use leptos::{prelude::*, task::spawn_local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::form_controls::{ConfigForm, PsfConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStatus {
    pub session_id: String,
    pub status: ProcessingStatus,
    pub message: String,
    pub progress: Option<f32>,
    pub images: Vec<GeneratedImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Idle,
    Processing,
    Complete,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub name: String,
    pub path: String,
    pub description: String,
}

#[component]
pub fn PsfGenerator() -> impl IntoView {
    let config = RwSignal::new(PsfConfig::default());
    let generation_status = RwSignal::new(GenerationStatus {
        session_id: String::new(),
        status: ProcessingStatus::Idle,
        message: String::new(),
        progress: None,
        images: Vec::new(),
    });

    let generate_psf = move || {
        leptos::logging::log!("generate_psf function called!");
        println!("generate psf");
        let config_value = config.get();
        let session_id = Uuid::new_v4().to_string();

        // First set a visible status to confirm button click worked
        generation_status.set(GenerationStatus {
            session_id: session_id.clone(),
            status: ProcessingStatus::Processing,
            message: "Button clicked! Sending request...".to_string(),
            progress: Some(0.0),
            images: Vec::new(),
        });

        spawn_local(async move {
            match generate_psf_request(config_value, session_id.clone()).await {
                Ok(status) => generation_status.set(status),
                Err(e) => generation_status.set(GenerationStatus {
                    session_id,
                    status: ProcessingStatus::Error,
                    message: format!("Error: {}", e),
                    progress: None,
                    images: Vec::new(),
                }),
            }
        });
    };

    view! {
        <div class="space-y-8">
            <ConfigForm config=config on_submit=generate_psf/>

            <StatusDisplay generation_status=generation_status/>

            <ImageGallery generation_status=generation_status/>
        </div>
    }
}

async fn generate_psf_request(
    config: PsfConfig,
    session_id: String,
) -> Result<GenerationStatus, String> {
    leptos::logging::log!("generate_psf_request called with config: {:?}", &config);
    println!("generate psf request");
    let response = Request::post("/api/generate")
        .header("Content-Type", "application/json") // Explicit header
        .json(&serde_json::json!({
            "config": config,
            "session_id": session_id
        }))
        .map_err(|e| format!("Failed to create request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    // Always try to parse the JSON response, even for errors
    let status = response
        .json::<GenerationStatus>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Check if it's an error status in the parsed response
    match status.status {
        ProcessingStatus::Error => Err(status.message),
        _ => Ok(status),
    }
}


#[component]
fn StatusDisplay(generation_status: RwSignal<GenerationStatus>) -> impl IntoView {
    view! {
        <div class="bg-gray-50 rounded-lg p-6">
            <h3 class="text-lg font-semibold mb-3 text-gray-800">"Generation Status"</h3>

            {move || {
                let status = generation_status.get();
                match status.status {
                    ProcessingStatus::Idle => view! {
                        <div class="flex items-center space-x-2">
                            <div class="w-3 h-3 bg-gray-400 rounded-full"></div>
                            <span class="text-gray-600">"Ready to generate PSF frames"</span>
                        </div>
                    }.into_any(),
                    ProcessingStatus::Processing => view! {
                        <div class="space-y-3">
                            <div class="flex items-center space-x-2">
                                <div class="w-3 h-3 bg-blue-500 rounded-full animate-pulse"></div>
                                <span class="text-blue-600 font-medium">"Processing..."</span>
                            </div>
                            <p class="text-gray-600 text-sm">{status.message}</p>
                            {status.progress.map(|progress| view! {
                                <div class="w-full bg-gray-200 rounded-full h-2">
                                    <div
                                        class="bg-blue-600 h-2 rounded-full transition-all duration-300"
                                        style=format!("width: {}%", progress)
                                    ></div>
                                </div>
                            })}
                        </div>
                    }.into_any(),
                    ProcessingStatus::Complete => view! {
                        <div class="flex items-center space-x-2">
                            <div class="w-3 h-3 bg-green-500 rounded-full"></div>
                            <span class="text-green-600 font-medium">"Generation complete!"</span>
                        </div>
                    }.into_any(),
                    ProcessingStatus::Error => view! {
                        <div class="space-y-2">
                            <div class="flex items-center space-x-2">
                                <div class="w-3 h-3 bg-red-500 rounded-full"></div>
                                <span class="text-red-600 font-medium">"Error occurred"</span>
                            </div>
                            <p class="text-red-600 text-sm bg-red-50 p-2 rounded">{status.message}</p>
                        </div>
                    }.into_any(),
                }
            }}
        </div>
    }
}

#[component]
fn ImageGallery(generation_status: RwSignal<GenerationStatus>) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow-md p-6">
            <h3 class="text-lg font-semibold mb-4 text-gray-800">"Generated Images"</h3>

            {move || {
                let status = generation_status.get();
                if status.images.is_empty() {
                    view! {
                        <p class="text-gray-500 text-center py-8">
                            "No images generated yet. Submit a configuration above to start."
                        </p>
                    }.into_any()
                } else {
                    view! {
                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                            {status.images.into_iter().map(|image| view! {
                                <div class="bg-gray-50 rounded-lg p-4">
                                    <img
                                        src={image.path.clone()}
                                        alt={image.name.clone()}
                                        class="w-full h-auto rounded-lg mb-3 shadow-sm"
                                    />
                                    <h4 class="font-medium text-gray-800 mb-1">{image.name.clone()}</h4>
                                    <p class="text-sm text-gray-600">{image.description}</p>
                                    <a
                                        href={image.path}
                                        download={image.name}
                                        class="inline-block mt-2 px-3 py-1 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 transition-colors"
                                    >
                                        "Download"
                                    </a>
                                </div>
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
