//NOTE: based on http://archive.gamedev.net/archive/reference/programming/features/beatdetection/

use std::sync::mpsc::*;
use portaudio;
use meyda; 

#[derive(Debug)]
struct State {
    energy_buffer: Vec<f64>,
    energy_average: f64,
    // scene_meshes: Vec<three::Mesh>
}

fn main() {
    let pa = portaudio::PortAudio::new().expect("Unable to init PortAudio"); 
    let mic_index = pa.default_input_device().expect("Unable to get default device");
    let mic = pa.device_info(mic_index).expect("unable to get mic info");

    let input_params = portaudio::StreamParameters::<f32>::new(mic_index, 1, true, mic.default_low_input_latency);
    let input_settings = portaudio::InputStreamSettings::new(input_params, mic.default_sample_rate, 1024);

    let (sender, receiver) = channel(); 
    let mut audio_buffer : &[f32] = &[]; 

    let callback = move |portaudio::InputStreamCallbackArgs {buffer, .. }| {
        audio_buffer = buffer;

        match sender.send(audio_buffer) {
            Ok(_) => portaudio::Continue, 
            Err(_) => portaudio::Complete
        }
    };

    let mut state = State {
        energy_buffer: Vec::new(),
        energy_average: 0.0,
        // scene_meshes: Vec::new()
    };

    let mut stream = pa.open_non_blocking_stream(input_settings, callback).expect("Unable to create stream"); 
    stream.start().expect("Unable to start stream"); 
    println!("opening stream..."); 

    loop {
        while let Ok(audio_buffer) = receiver.try_recv() {
            update_sound_values(&audio_buffer, &mut state, mic.default_sample_rate); 
        }
    }
}

fn update_sound_values(samples: &[f32], state: &mut State, sample_rate: f64) {
   let signal: Vec<f64> = samples.iter().map(|sample| *sample as f64).collect(); 
   let energy_buffer = meyda::get_energy(&signal); 

   let energy_sum = state.energy_buffer.iter().fold(0.0, |sum, energy| sum + energy );
   state.energy_average = energy_sum / (state.energy_buffer.len() as f64); 

   if energy_buffer > state.energy_average * 2.0 {
       println!("beat {:?}", energy_buffer); 
   }
   
   state.energy_buffer.push(energy_buffer); 

   //keep no more than 1 sec worth of history
   if state.energy_buffer.len() > sample_rate as usize {
       state.energy_buffer.remove(0); 
   }
}