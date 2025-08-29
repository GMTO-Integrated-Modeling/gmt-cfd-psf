use std::{collections::HashMap, fmt::Display};

use leptos::prelude::Show;
use leptos::prelude::*;
use psf::{get_enclosure_config, AzimuthAngle, ElevationAngle, WindSpeed, ZenithAngle};
use serde::{Deserialize, Serialize};

use crate::components::youtube_playlists;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RbmTimeSeries {
    OpenLoop,
    Fsm,
    Asm,
}
impl Display for RbmTimeSeries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::OpenLoop => "m1_m2_rbms.parquet",
                Self::Fsm => "m1_m2_rbms.FSM.parquet",
                Self::Asm => "m1_m2_rbms.ASM.2.parquet",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PsfConfig {
    pub domeseeing: bool,
    pub windloads: bool,
    pub elevation_angle: ElevationAngle,
    pub azimuth_angle: AzimuthAngle,
    pub wind_speed: WindSpeed,
    pub rbm_time_series: RbmTimeSeries,
}

impl Default for PsfConfig {
    fn default() -> Self {
        Self {
            domeseeing: false,
            windloads: false,
            elevation_angle: ElevationAngle::Sixty,
            azimuth_angle: AzimuthAngle::Zero,
            wind_speed: WindSpeed::Seven,
            rbm_time_series: RbmTimeSeries::OpenLoop
        }
    }
}

#[component]
pub fn CfdData(config: RwSignal<PsfConfig>) -> impl IntoView {
    // Function to generate YouTube video title based on configuration
    let domeseeing_playlist: HashMap<String, String> =
        serde_json::from_str(youtube_playlists::DOMESEEING).unwrap();
    let (domeseeing_playlist, ..) = signal(domeseeing_playlist);
    let get_domeseeing_video = move || {
        let cfg = config.get();
        let zenith_str = format!("{:02}", ZenithAngle::from(cfg.elevation_angle).as_u32());
        let azimuth_str = format!("{:03}", cfg.azimuth_angle.as_u32());
        let enclosure =
            get_enclosure_config(cfg.wind_speed.as_u32(), cfg.elevation_angle).to_uppercase();
        let wind_speed = cfg.wind_speed.as_u32();
        let title = format!(
            "zen{}az{}_{}_{wind_speed}ms",
            zenith_str, azimuth_str, enclosure
        );
        let id = domeseeing_playlist.get().get(&title).unwrap().to_owned();
        (title, id)
    };

    let windloads_playlist: HashMap<String, String> =
        serde_json::from_str(youtube_playlists::WINDLOADS).unwrap();
    let (windloads_playlist, ..) = signal(windloads_playlist);
    let get_windloads_video = move || {
        let cfg = config.get();
        let zenith_str = format!("{:02}", ZenithAngle::from(cfg.elevation_angle).as_u32());
        let azimuth_str = format!("{:03}", cfg.azimuth_angle.as_u32());
        let enclosure =
            get_enclosure_config(cfg.wind_speed.as_u32(), cfg.elevation_angle).to_uppercase();
        let wind_speed = cfg.wind_speed.as_u32();
        let title = format!(
            "zen{}az{}_{}_{wind_speed}ms",
            zenith_str, azimuth_str, enclosure
        );
        let id = windloads_playlist.get().get(&title).unwrap().to_owned();
        (title, id)
    };

    view! {
                <fieldset class="border border-gray-300 rounded-lg p-4">
                    <legend class="text-lg font-medium text-gray-700 px-2">
                        "CFD Data"
                    </legend>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-2">
                        <label class="flex items-center space-x-2">
                            <input
                                type="checkbox"
                                checked=move || config.get().domeseeing
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    config.update(|c| c.domeseeing = checked);
                                }
                                class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                            />
                            <span class="text-sm font-medium text-gray-700">"Dome Seeing"</span>
                        </label>

                        <div class="flex items-center space-x-4">
                            <label class="flex items-center space-x-2">
                                <input
                                    type="checkbox"
                                    checked=move || config.get().windloads
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        config.update(|c| c.windloads = checked);
                                    }
                                    class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                                />
                                <span class="text-sm font-medium text-gray-700">"Wind Loads"</span>
                            </label>
                            
                            <select
                                class="p-1 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 text-sm"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    let rbm_series = match value.as_str() {
                                        "OpenLoop" => RbmTimeSeries::OpenLoop,
                                        "Fsm" => RbmTimeSeries::Fsm,
                                        "Asm" => RbmTimeSeries::Asm,
                                        _ => RbmTimeSeries::OpenLoop,
                                    };
                                    config.update(|c| c.rbm_time_series = rbm_series);
                                }
                            >
                                <option 
                                    value="OpenLoop" 
                                    selected=move || matches!(config.get().rbm_time_series, RbmTimeSeries::OpenLoop)
                                >
                                    "open-loop"
                                </option>
                                <option 
                                    value="Fsm" 
                                    selected=move || matches!(config.get().rbm_time_series, RbmTimeSeries::Fsm)
                                >
                                    "closed-loop FSM"
                                </option>
                                <option 
                                    value="Asm" 
                                    selected=move || matches!(config.get().rbm_time_series, RbmTimeSeries::Asm)
                                >
                                    "closed-loop ASM"
                                </option>
                            </select>
                        </div>
                    </div>

                    // YouTube videos section - side by side layout
                    <Show when=move || config.get().domeseeing || config.get().windloads>
                        <div class="mt-4 border-t border-gray-200 pt-4">
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                // DomeSeeing video (left side)
                                <Show when=move || config.get().domeseeing>
                                    {move || {
                                        let (video_title, video_id) = get_domeseeing_video();
                                        view! {
                                            <div>
                                                <h4 class="text-md font-medium text-gray-700 mb-3">
                                                    "Gradient of the Index of Refraction"
                                                </h4>
                                                <div class="relative w-full" style="padding-bottom: 56.25%;">
                                                    <iframe
                                                        class="absolute top-0 left-0 w-full h-full rounded-lg shadow-md"
                                                        src=format!("https://www.youtube.com/embed/{video_id}")
                                                        title=format!("CFD Data Visualization: {}", video_title)
                                                        style="border: 0;"
                                                        allow=" clipboard-write; encrypted-media; picture-in-picture"
                                                        allowfullscreen=true
                                                    >
                                                    </iframe>
                                                </div>
                                            </div>
                                        }
                                    }}
                                </Show>

                                // Windloads video (right side)
                                <Show when=move || config.get().windloads>
                                    {move || {
                                        let (video_title, video_id) = get_windloads_video();
                                        view! {
                                            <div>
                                                <h4 class="text-md font-medium text-gray-700 mb-3">
                                                    "Vorticity"
                                                </h4>
                                                <div class="relative w-full" style="padding-bottom: 56.25%;">
                                                    <iframe
                                                        class="absolute top-0 left-0 w-full h-full rounded-lg shadow-md"
                                                        src=format!("https://www.youtube.com/embed/{video_id}")
                                                        title=format!("CFD Data Visualization: {}", video_title)
                                                        style="border: 0;"
                                                        allow=" clipboard-write; encrypted-media; picture-in-picture"
                                                        allowfullscreen=true
                                                    >
                                                    </iframe>
                                                </div>
                                            </div>
                                        }
                                    }}
                                </Show>
                            </div>
                        </div>
                    </Show>
                </fieldset>
    }
}
#[component]
pub fn ElevationAngle(config: RwSignal<PsfConfig>) -> impl IntoView {
    let get_zenith_image = |angle: &ElevationAngle| -> &'static str {
        match angle {
            ElevationAngle::Ninety => "/assets/zen00az000_OS7_tel_tr.png",
            ElevationAngle::Thirty => "/assets/zen60az000_CS17_tel_tr.png",
            ElevationAngle::Sixty => "/assets/zen30az000_CD12_tel_tr.png",
        }
    };

    view! {
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-2">
                                "Telescope elevation"
                            </label>
                            <div class="mt-2">
                                <img
                                    src=move || get_zenith_image(&config.get().elevation_angle)
                                    alt=move || format!("Zenith angle {} illustration", config.get().elevation_angle.as_str())
                                    class="h-auto rounded border border-gray-200"
                                    style="width: 55%"
                                />
                            </div>
                            <select
                                class="w-full p-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 mb-2"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    let zenith = match value.as_str() {
                                        "90" => ElevationAngle::Ninety,
                                        "60" => ElevationAngle::Sixty,
                                        "30" => ElevationAngle::Thirty,
                                        _ => ElevationAngle::Thirty,
                                    };
                                    config.update(|c| c.elevation_angle = zenith);
                                }
                            >
                                {ElevationAngle::all().into_iter().map(|angle| {
                                    let selected = move || config.get().elevation_angle == angle;
                                    view! {
                                        <option
                                            value={angle.as_u32().to_string()}
                                            selected=selected
                                        >
                                            {angle.as_str()}
                                        </option>
                                    }
                                }).collect::<Vec<_>>()}
                            </select>
                        </div>

    }
}

#[component]
pub fn AzimuthAngle(config: RwSignal<PsfConfig>) -> impl IntoView {
    let get_azimuth_image = |angle: &AzimuthAngle| -> &'static str {
        match angle {
            AzimuthAngle::Zero => "/assets/az0.png",
            AzimuthAngle::FortyFive => "/assets/az1.png",
            AzimuthAngle::Ninety => "/assets/az2.png",
            AzimuthAngle::OneThirtyFive => "/assets/az3.png",
            AzimuthAngle::OneEighty => "/assets/az4.png",
        }
    };

    view! {
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
                "Telescope relative to wind"
            </label>
            <div class="mt-2">
                <img
                    src=move || get_azimuth_image(&config.get().azimuth_angle)
                    alt=move || format!("Azimuth angle {} illustration", config.get().azimuth_angle.as_str())
                    class="w-full h-auto rounded border border-gray-200"
                />
            </div>
            <select
                class="w-full p-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 mb-2"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    let azimuth = match value.as_str() {
                        "0" => AzimuthAngle::Zero,
                        "45" => AzimuthAngle::FortyFive,
                        "90" => AzimuthAngle::Ninety,
                        "135" => AzimuthAngle::OneThirtyFive,
                        "180" => AzimuthAngle::OneEighty,
                        _ => AzimuthAngle::Zero,
                    };
                    config.update(|c| c.azimuth_angle = azimuth);
                }
            >
                {AzimuthAngle::all().into_iter().map(|angle| {
                    let selected = move || config.get().azimuth_angle == angle;
                    view! {
                        <option
                            value={angle.as_u32().to_string()}
                            selected=selected
                        >
                            {angle.as_str()}
                        </option>
                    }
                }).collect::<Vec<_>>()}
            </select>
        </div>
    }
}

#[component]
pub fn WindSpeed(config: RwSignal<PsfConfig>) -> impl IntoView {
    view! {
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-2">
                        "Wind speed"
                    </label>
                    <div class="mt-2">
                        <img
                            src=move || {
                                let cfg = config.get();
                                get_enclosure_image(cfg.wind_speed.as_u32(), cfg.elevation_angle)
                            }
                            alt=move || {
                                let cfg = config.get();
                                let enclosure = get_enclosure_config(cfg.wind_speed.as_u32(), cfg.elevation_angle);
                                format!("Enclosure configuration: {}", enclosure)
                            }
                            class="h-auto rounded border border-gray-200"
                            style="width: 65%"
                        />
                    </div>
                    <select
                        class="w-full p-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            let wind = match value.as_str() {
                                "2" => WindSpeed::Two,
                                "7" => WindSpeed::Seven,
                                "12" => WindSpeed::Twelve,
                                "17" => WindSpeed::Seventeen,
                                _ => WindSpeed::Seven,
                            };
                            config.update(|c| c.wind_speed = wind);
                        }
                    >
                        {WindSpeed::all().into_iter().map(|speed| {
                            let selected = move || config.get().wind_speed == speed;
                            view! {
                                <option
                                    value={speed.as_u32().to_string()}
                                    selected=selected
                                >
                                    {speed.as_str()}
                                </option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>
                </div>
    }
}

fn get_vents_status(wind_speed: u32, pointing: impl Into<ZenithAngle>) -> &'static str {
    let enclosure_config = get_enclosure_config(wind_speed, pointing);
    match enclosure_config {
        "os" => "open",
        "cd" | "cs" => "closed",
        _ => "closed",
    }
}

fn get_wind_screen_status(wind_speed: u32, pointing: impl Into<ZenithAngle>) -> &'static str {
    let enclosure_config = get_enclosure_config(wind_speed, pointing);
    match enclosure_config {
        "os" | "cs" => "stowed",
        "cd" => "deployed",
        _ => "stowed",
    }
}

fn get_enclosure_image(wind_speed: u32, pointing: impl Into<ZenithAngle>) -> &'static str {
    let enclosure_config = get_enclosure_config(wind_speed, pointing);
    match enclosure_config {
        "os" => "/assets/zen30az000_OS7_tr.png",
        "cd" => "/assets/zen30az000_CD12_tr.png",
        "cs" => "/assets/zen60az000_CS17_tr.png",
        _ => "/assets/zen30az000_OS7_tr.png",
    }
}

#[component]
pub fn Vents(config: RwSignal<PsfConfig>) -> impl IntoView {
    let vents_status = move || {
        let cfg = config.get();
        get_vents_status(cfg.wind_speed.as_u32(), cfg.elevation_angle)
    };

    view! {
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
                "Vents"
            </label>
            <div class="mt-2">
                <img
                    src=move || {
                        let cfg = config.get();
                        get_enclosure_image(cfg.wind_speed.as_u32(), cfg.elevation_angle)
                    }
                    alt=move || {
                        let cfg = config.get();
                        let enclosure = get_enclosure_config(cfg.wind_speed.as_u32(), cfg.elevation_angle);
                        format!("Enclosure configuration: {}", enclosure)
                    }
                    class="h-auto rounded border border-gray-200"
                    style="width: 65%"
                />
            </div>
            <input
                type="text"
                value=vents_status
                readonly
                class="w-full p-2 border border-gray-300 rounded-md bg-gray-50 text-gray-600 mb-2"
            />
        </div>
    }
}

#[component]
pub fn WindScreen(config: RwSignal<PsfConfig>) -> impl IntoView {
    let wind_screen_status = move || {
        let cfg = config.get();
        get_wind_screen_status(cfg.wind_speed.as_u32(), cfg.elevation_angle)
    };

    view! {
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
                "Wind Screen"
            </label>
            <div class="mt-2">
                <img
                    src=move || {
                        let cfg = config.get();
                        get_enclosure_image(cfg.wind_speed.as_u32(), cfg.elevation_angle)
                    }
                    alt=move || {
                        let cfg = config.get();
                        let enclosure = get_enclosure_config(cfg.wind_speed.as_u32(), cfg.elevation_angle);
                        format!("Enclosure configuration: {}", enclosure)
                    }
                    class="h-auto rounded border border-gray-200"
                    style="width: 65%"
                />
            </div>
            <input
                type="text"
                value=wind_screen_status
                readonly
                class="w-full p-2 border border-gray-300 rounded-md bg-gray-50 text-gray-600 mb-2"
            />
        </div>
    }
}

#[component]
pub fn ConfigForm(config: RwSignal<PsfConfig>, on_submit: impl Fn() + 'static) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow-md p-6">
            <h2 class="text-2xl font-semibold mb-4 text-gray-800">
                "PSF Configuration"
            </h2>

            <form on:submit=move |ev| {
                ev.prevent_default();
                on_submit();
            } class="space-y-6">

                // CFD Data Section
                <CfdData config=config/>

                // Telescope Configuration Section
                <fieldset class="border border-gray-300 rounded-lg p-4">
                    <legend class="text-lg font-medium text-gray-700 px-2">
                        "CFD Configuration"
                    </legend>
                        <div class="mb-4">
                            <p class="text-sm text-gray-600 bg-blue-50 border-l-4 border-blue-400 p-3 rounded">
                                "Select the telescope orientation with respect to the wind, the wind is always blowing from the NNE direction; Select the wind speed, the enclosure vents and wind screen setup depends on the wind speed: either open/stowed or closed/deployed; Select the telescope elevation angle (at low elevation the wind screen is stowed)"
                            </p>
                        </div>
                    <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-5 gap-4 mt-2">

                        // Azimuth Angle
                        <AzimuthAngle config=config/>

                        // Wind Speed
                        <WindSpeed config=config/>

                        // Zenith Angle
                        <ElevationAngle config=config/>

                        // // Vents
                        // <Vents config=config/>

                        // // Wind Screen
                        // <WindScreen config=config/>
                    </div>
                </fieldset>

                // Submit Button
                <div class="flex justify-center">
                    <button
                        type="submit"
                        class="px-8 py-3 bg-blue-600 hover:bg-blue-700 text-white font-semibold rounded-lg shadow-md transition-colors duration-200"
                    >
                        "Generate PSF Frames"
                    </button>
                </div>
            </form>
        </div>
    }
}
