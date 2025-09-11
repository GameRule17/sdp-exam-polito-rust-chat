/*
Modulo Util: contiene funzioni di utilità generali
in questo caso contiene solo la funzione per generare codici invito a 6 caratteri
*/

// codice invito per il gruppo (6 caratteri alfanumerici)
pub fn short_code() -> String {
    use rand::{distributions::Alphanumeric, Rng};
    let mut rng = rand::thread_rng();
    (0..6).map(|_| rng.sample(Alphanumeric) as char).collect()
}
