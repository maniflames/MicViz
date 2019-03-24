use portaudio;
use std::sync::mpsc::*;
                                                                                                                                                                        
fn main() {
    let pa = portaudio::PortAudio::new().expect("Unable to init PortAudio"); 
    let mic_index = pa.default_input_device().expect("Unable to get default device");
    let mic = pa.device_info(mic_index).expect("unable to get mic info");

    let input_params = portaudio::StreamParameters::<f32>::new(mic_index, 1, true, mic.default_low_input_latency);
    let input_settings = portaudio::InputStreamSettings::new(input_params, mic.default_sample_rate, 256);

    let (sender, receiver) = channel(); 
    let mut audio_buffer : &[f32] = &[]; 

    let callback = move |portaudio::InputStreamCallbackArgs {buffer, .. }| {
        audio_buffer = buffer;

        match sender.send(audio_buffer) {
            Ok(_) => portaudio::Continue, 
            Err(_) => portaudio::Complete
        }
    };

    let mut stream = pa.open_non_blocking_stream(input_settings, callback).expect("Unable to create stream"); 
    stream.start().expect("Unable to start stream"); 

    while stream.is_active().unwrap() {
       while let Ok(audio_buffer) = receiver.try_recv() {
            print_audio_samples(audio_buffer); 
       }
    }
}

fn print_audio_samples(samples: &[f32]) {
    println!("{:?}", samples);
}