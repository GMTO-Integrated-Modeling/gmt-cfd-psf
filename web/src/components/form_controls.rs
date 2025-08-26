use leptos::prelude::*;
use psf::{AzimuthAngle, WindSpeed, ZenithAngle, get_enclosure_config};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PsfConfig {
    pub domeseeing: bool,
    pub windloads: bool,
    pub zenith_angle: ZenithAngle,
    pub azimuth_angle: AzimuthAngle,
    pub wind_speed: WindSpeed,
}

impl Default for PsfConfig {
    fn default() -> Self {
        Self {
            domeseeing: false,
            windloads: false,
            zenith_angle: ZenithAngle::Thirty,
            azimuth_angle: AzimuthAngle::Zero,
            wind_speed: WindSpeed::Seven,
        }
    }
}

#[component]
pub fn CfdData(config: RwSignal<PsfConfig>) -> impl IntoView {
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
                    </div>
                </fieldset>
    }
}
#[component]
pub fn ZenithAngle(config: RwSignal<PsfConfig>) -> impl IntoView {
    let get_zenith_image = |angle: &ZenithAngle| -> &'static str {
        match angle {
            ZenithAngle::Zero => "/assets/zen00az000_OS7_tel_tr.png",
            ZenithAngle::Thirty => "/assets/zen30az000_CD12_tel_tr.png", 
            ZenithAngle::Sixty => "/assets/zen60az000_CS17_tel_tr.png",
        }
    };

    view! {
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-2">
                                "Telescope pointing zenith angle"
                            </label>
                            <div class="mt-2">
                                <img
                                    src=move || get_zenith_image(&config.get().zenith_angle)
                                    alt=move || format!("Zenith angle {} illustration", config.get().zenith_angle.as_str())
                                    class="h-auto rounded border border-gray-200"
                                    style="width: 55%"
                                />
                            </div>
                            <select
                                class="w-full p-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 mb-2"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    let zenith = match value.as_str() {
                                        "0" => ZenithAngle::Zero,
                                        "30" => ZenithAngle::Thirty,
                                        "60" => ZenithAngle::Sixty,
                                        _ => ZenithAngle::Thirty,
                                    };
                                    config.update(|c| c.zenith_angle = zenith);
                                }
                            >
                                {ZenithAngle::all().into_iter().map(|angle| {
                                    let selected = move || config.get().zenith_angle == angle;
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

fn get_vents_status(wind_speed: u32, zenith_angle: u32) -> &'static str {
    let enclosure_config = get_enclosure_config(wind_speed, zenith_angle);
    match enclosure_config {
        "os" => "open",
        "cd" | "cs" => "closed",
        _ => "closed",
    }
}

fn get_wind_screen_status(wind_speed: u32, zenith_angle: u32) -> &'static str {
    let enclosure_config = get_enclosure_config(wind_speed, zenith_angle);
    match enclosure_config {
        "os" | "cs" => "stowed",
        "cd" => "deployed", 
        _ => "stowed",
    }
}

fn get_enclosure_image(wind_speed: u32, zenith_angle: u32) -> &'static str {
    let enclosure_config = get_enclosure_config(wind_speed, zenith_angle);
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
        get_vents_status(cfg.wind_speed.as_u32(), cfg.zenith_angle.as_u32())
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
                        get_enclosure_image(cfg.wind_speed.as_u32(), cfg.zenith_angle.as_u32())
                    }
                    alt=move || {
                        let cfg = config.get();
                        let enclosure = get_enclosure_config(cfg.wind_speed.as_u32(), cfg.zenith_angle.as_u32());
                        format!("Enclosure configuration: {}", enclosure)
                    }
                    class="h-auto rounded border border-gray-200"
                    style="width: 60%"
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
        get_wind_screen_status(cfg.wind_speed.as_u32(), cfg.zenith_angle.as_u32())
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
                        get_enclosure_image(cfg.wind_speed.as_u32(), cfg.zenith_angle.as_u32())
                    }
                    alt=move || {
                        let cfg = config.get();
                        let enclosure = get_enclosure_config(cfg.wind_speed.as_u32(), cfg.zenith_angle.as_u32());
                        format!("Enclosure configuration: {}", enclosure)
                    }
                    class="h-auto rounded border border-gray-200"
                    style="width: 60%"
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
                    <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-5 gap-4 mt-2">

                        // Zenith Angle
                        <ZenithAngle config=config/>

                        // Azimuth Angle
                        <AzimuthAngle config=config/>

                        // Wind Speed
                        <WindSpeed config=config/>

                        // Vents
                        <Vents config=config/>

                        // Wind Screen
                        <WindScreen config=config/>
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
