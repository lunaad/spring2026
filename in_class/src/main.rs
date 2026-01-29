
fn main() {
    // we  are going to accept a letter like RGB
    // and we should return 
    // Red tuple (255,0, 0)
    // Green tuple (0, 255, 0)
    // Blue tuple (0, 0, 255)

    // write a function which accepts a char 'R', 'G', 'B'
    // and return above specified tuple
    // let res = get_rgb('R');
    // println!("{:?}", res);
    // let res = get_rgb('G');
    // println!("{:?}", res);
    // let res = get_rgb('B');
    // println!("{:?}", res);
    // let letters = ['R', 'G', 'B'];

    // for l in letters.iter(){
    //     let res = get_rgb(*l);
    //     println!("{:?}", res);
    // }

    let letters = ['R', 'G', 'B'];

    for idx in 0..letters.len() {
        let res = get_rgb(letters[idx]);
        println!("{:?}", res);
    }

fn get_rgb(c: char) -> (u8, u8, u8) {
    // if c == 'R' {
    //     (255, 0, 0)
    // } 

    //  if c == 'G' {
    //     (0, 255, 0)
    // } 

    //  if c == 'B' {
    //     (0, 0, 255)
    // } 
    // (0, 0, 0) // default case for invalid input

    match c {
        'R' => (255, 0, 0),
        'G' => (0, 255, 0),
        'B' => (0, 0, 255),
        _ => (0, 0, 0), // default case for invalid input
    }
}
