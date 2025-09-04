/*
Modulo Util: contiene funzioni di utilità generali, come la generazione di codici invito brevi.
*/

// codice invito breve (6 char alfanumerici)
pub fn short_code() -> String {
    use rand::{distributions::Alphanumeric, Rng};
    let mut rng = rand::thread_rng();
    (0..6).map(|_| rng.sample(Alphanumeric) as char).collect()
}
