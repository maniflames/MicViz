use portaudio;
use std::sync::mpsc::*;
use three;
use std::collections::VecDeque;

//I should really consiuder putting this into a seperate crate because the scope isn't that small lmao

//NOTE: used research, video form https://www.youtube.com/watch?v=FmwpkdcAXl0&list=WL&index=152&t=0s
//NOTE: powerpoint form: https://www.audiolabs-erlangen.de/resources/MIR/2017-GI-Tutorial-Musik/2017_MuellerWeissBalke_GI_BeatTracking.pdf
//NOTE: paper form: https://ieeexplore-ieee-org.ezproxy.hro.nl/stamp/stamp.jsp?tp=&arnumber=5654580
//NOTE: use this to navigate the paper https://www.rapidtables.com/math/symbols/Basic_Math_Symbols.html

//NOTE: use parts of https://github.com/meyda/meyda-rs
//NOTE: original lib docs https://meyda.js.org/audio-features
//NOTE: alternative implemented in JS https://jmperezperez.com/bpm-detection-javascript/

//novelty curve => https://musicinformationretrieval.com/novelty_functions.html
// python calls this "spectral flux" https://librosa.github.io/librosa/generated/librosa.onset.onset_strength.html
    //1. spectogram (signal to time frequency aka spectogram aka (Fast) Fourier Transform) -> w/ meyda::get_amp_spectrum()
    //NOTE: learn about FFT https://www.mathworks.com/help/signal/examples/practical-introduction-to-frequency-domain-analysis.html
    //2. log compression (apply logarithm Y = log(1 + C * |X|) => this might be power spectrum in meyda but I'm unsure otherwise I have to implement LFT =>  POWER-SCALED SPECTRAL FLUX !!!
    //3. differentiation, discrete derivative https://calculus.subwiki.org/wiki/Discrete_derivative, sperately for each frequency band, consider only positive differences, should probably be forward => maybe use specteral flux
    //(specteral difference => tbh there are several research papers on this, i.e. http://www.cerfacs.fr/~cfdbib/repository/WN_CFD_12_57.pdf)
    //4. accumulation, sum up all positive differences so each 'frame' has a single positive number
    //5. normalisation compute a local average curve and subtract from the curve that was outputed by the accumulation step
    //output = noverlty curve 

//pick the peaks and you have the simplest form of onset detection!

//Meyda can do all of this but calls it 'spectral flux' instead of novelty curve T-T??? => https://meyda.js.org/audio-features => almost the same, 
//in meyda they aren't making a difference between the difference frequencies, I can steal some of the math though, basically do what they do for each requency band https://github.com/meyda/meyda/blob/master/src/extractors/spectralFlux.js
//librosa has straight up onset detection => https://librosa.github.io/librosa/generated/librosa.onset.onset_strength.html

//SERIOUSLY A GUY WAS WORKING ON THIS BUT STOPPED WHY?!!? https://github.com/ahdinosaur/beat-playground
//another research paper => http://mac.citi.sinica.edu.tw/~yang/pub/su14smc_onset.pdf nr 3. =  POWER-SCALED SPECTRAL FLUX 

//a little more detail about onset detection http://resources.mpi-inf.mpg.de/departments/d4/teaching/ss2010/mp_mm/2010_MuellerGrosche_Lecture_MusicProcessing_BeatTracking_handout.pdf

//TODO: print something on onset
//TODO: create visualisation on onset
fn main() {
    let pa = portaudio::PortAudio::new().expect("Unable to open PortAudio"); 
    let default_mic_index = pa.default_input_device().expect("Unable to get default device"); 
    let mic = pa.device_info(default_mic_index).expect("Unable to get mic info"); 

    let input_stream_params = portaudio::StreamParameters::<f32>::new(default_mic_index, 1, true, mic.default_low_input_latency);
    let input_stream_settings = portaudio::InputStreamSettings::new(input_stream_params, mic.default_sample_rate, 256);

    let (sender, receiver) = channel();

    let mut stream = pa.open_non_blocking_stream(input_stream_settings, move |portaudio::InputStreamCallbackArgs {buffer, ..}| {
        //samples vs signal?? for namin variables sake what am I sending? (seems like a sample = frame of audio = signal)
        match sender.send(buffer) {
            Ok(_) => portaudio::Continue,
            Err(_) => portaudio::Complete
        }
    }).expect("Unable to open stream");

    println!("Starting audio stream...");
    stream.start().expect("Unable to start stream"); 

    let mut history: VecDeque<Vec<f64>> = VecDeque::new(); //aka spectrum_history
    let mut novelty_history: VecDeque<f64> = VecDeque::new(); 
    let mut normalised_novelty_history: VecDeque<f64> = VecDeque::new(); 
    let mut garbage_collection: Vec<three::Mesh> = Vec::new(); 

    let mut builder = three::Window::builder("A window Imani built"); 
    builder.fullscreen(true); 
    let mut win = builder.build(); 
    win.scene.background = three::Background::Color(0x000000);

    let camera = win.factory.orthographic_camera([0.0, 0.0], 1.0, -1.0 .. 1.0); 

    while win.update() {
        match receiver.try_recv() {
            Ok(buffer) => calculate_novelty_curve(buffer, &mut history, &mut novelty_history, &mut normalised_novelty_history), 
            Err(_err) => ()
        }
        draw_curve(&mut novelty_history, &mut normalised_novelty_history, &mut win, &mut garbage_collection);
        win.render(&camera); 
        remove_lines(&mut win, &mut garbage_collection);
        garbage_collection.clear(); 
    }  
}

fn calculate_novelty_curve(buffer: &[f32], history: &mut VecDeque<Vec<f64>>, novelty_history: &mut VecDeque<f64>, normalised_novelty_history: &mut VecDeque<f64>) {
    let samples: Vec<f64> = buffer.to_vec().into_iter().map(|sample| sample as f64).collect(); 
    //Fourier Transform
    let spectrum = meyda::get_amp_spectrum(&samples);

    //Log compression
    // Y = log( 1 + C * |X|) 
    let log_spectrum: Vec<f64> = spectrum.into_iter().map(|sample| {
            let to_compress = 1.0 + (1000.0 * sample);
            return to_compress.log10();
        }).collect(); 

    history.push_front(log_spectrum); 

    if history.len() < 2 {
        return
    }

    //differentiation: history[1] - history[0], negatives are dropped
    let mut differentiation: Vec<f64> = Vec::new(); 
    for (index, sample) in history[1].iter().enumerate() {
        let difference = sample - history[0][index]; 
        if difference >= 0.0 {
            differentiation.push(difference);
        }
    } 

    //remove unneeded history
    history.pop_back();

    //accumulation into novelty point
    let novelty_point = differentiation.iter().fold(0.0, |sum, difference| sum + difference);
    novelty_history.push_front(novelty_point);

    if novelty_history.len() < 250 {
        return
    }

    let local_average = novelty_history.iter().fold(0.0, |sum, novelty_point| sum + novelty_point) / (novelty_history.len() as f64);
    let novelty_history_loop = novelty_history.clone(); //this is a memory hack and should be fixed dureing refactor
    //instead of picking peaks I just use a treshold
    let treshold = 0.0; 

    normalised_novelty_history.clear();
    for novelty_point in novelty_history_loop {
        let candidate = novelty_point - local_average;
        if candidate < treshold {
            normalised_novelty_history.push_front(0.0);
            continue;
        }

        normalised_novelty_history.push_front(candidate);
    }

    //remove unneeded history
    novelty_history.pop_back();
}

//TODO: return meshes that need to be removed 
fn draw_curve(novelty_curve: &mut VecDeque<f64>, normalised_novelty_curve: &mut VecDeque<f64>, win: &mut three::window::Window, garbage_collection: &mut Vec<three::Mesh>) {
    let curve = normalised_novelty_curve; 

    for (index, novelty_point) in curve.iter().enumerate() {
        if index == 0 {
            continue;
        }

        let novelty_curve_len = curve.len() as f32; 
        let previous_index = (index - 1) as f32;
        let previous_x = ((previous_index / novelty_curve_len) * 3.0) - 1.5; 
        let previous_y = curve[index - 1] as f32 / 100.0; 
        let x = (((index as f32) / novelty_curve_len) * 3.0) - 1.5;
        let y = *novelty_point as f32 / 100.0; 

        //x from left to right is -1.5 to 1.5
        //y from bottom to top is 0.0 to 1.0
        let geometry = three::Geometry::with_vertices(vec![
            [previous_x, previous_y - 0.5, 0.0].into(),
            [x, y - 0.5, 0.0].into()
        ]);

        let material = three::material::Line {
            color: 0xFFFFFF,
        };

        let mesh = win.factory.mesh(geometry, material);
        win.scene.add(&mesh); 
        garbage_collection.push(mesh); 
    }
}

fn remove_lines(win: &mut three::window::Window, garbage_collection: &mut Vec<three::Mesh>) {
    for mesh in garbage_collection {
        win.scene.remove(&mesh); 
    }
}