use leptos::prelude::*;
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ZenithAngle {
    Zero = 0,
    Thirty = 30,
    Sixty = 60,
}

impl ZenithAngle {
    pub fn all() -> Vec<Self> {
        vec![Self::Zero, Self::Thirty, Self::Sixty]
    }

    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Zero => "0°",
            Self::Thirty => "30°",
            Self::Sixty => "60°",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AzimuthAngle {
    Zero = 0,
    FortyFive = 45,
    Ninety = 90,
    OneThirtyFive = 135,
    OneEighty = 180,
}

impl AzimuthAngle {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Zero,
            Self::FortyFive,
            Self::Ninety,
            Self::OneThirtyFive,
            Self::OneEighty,
        ]
    }

    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Zero => "0°",
            Self::FortyFive => "45°",
            Self::Ninety => "90°",
            Self::OneThirtyFive => "135°",
            Self::OneEighty => "180°",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WindSpeed {
    Two = 2,
    Seven = 7,
    Twelve = 12,
    Seventeen = 17,
}

impl WindSpeed {
    pub fn all() -> Vec<Self> {
        vec![Self::Two, Self::Seven, Self::Twelve, Self::Seventeen]
    }

    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Two => "2 m/s",
            Self::Seven => "7 m/s",
            Self::Twelve => "12 m/s",
            Self::Seventeen => "17 m/s",
        }
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

                // Turbulence Effects Section
                <fieldset class="border border-gray-300 rounded-lg p-4">
                    <legend class="text-lg font-medium text-gray-700 px-2">
                        "Turbulence Effects"
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

                // Telescope Configuration Section
                <fieldset class="border border-gray-300 rounded-lg p-4">
                    <legend class="text-lg font-medium text-gray-700 px-2">
                        "Telescope Configuration"
                    </legend>
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-2">

                        // Zenith Angle
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-2">
                                "Zenith Angle"
                            </label>
                            <select
                                class="w-full p-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
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

                        // Azimuth Angle
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-2">
                                "Azimuth Angle"
                            </label>
                            <select
                                class="w-full p-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
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

                        // Wind Speed
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-2">
                                "Wind Speed"
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
