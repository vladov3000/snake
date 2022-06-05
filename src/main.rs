use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{WindowCanvas, Texture, TextureCreator};
use rand::rngs::ThreadRng;
use rand::Rng;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::ops::Add;
use std::collections::VecDeque;
use sdl2::ttf::Font;
use sdl2::mouse::MouseButton;
use crate::State::Running;
use std::process::exit;
use sdl2::rwops::RWops;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 400;

fn scale_by(x: u32, scale: f32) -> u32 {
    (x as f32 * scale) as u32
}

#[derive(PartialEq, Copy, Clone)]
struct Cell(i32, i32);

impl Add for Cell {
    type Output = Cell;

    fn add(self, rhs: Self) -> Self::Output {
        Cell(self.0 + rhs.0, self.1 + rhs.1)
    }
}

struct RunningGame {
    rng: ThreadRng,
    width: i32,
    height: i32,
    snake: VecDeque<Cell>,
    apple: Cell,
    direction: Direction,
    move_delay_ms: u64,
    last_move_time_ms: Option<u64>,
    game_over: bool,
    score: i32,
}

impl RunningGame {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        let width = 10;
        let height = 10;
        let apple = Self::make_apple(width, height, &mut rng);

        RunningGame {
            rng,
            width,
            height,
            snake: VecDeque::from(vec![Cell(width / 2, height / 2)]),
            apple,
            direction: Direction::Down,
            move_delay_ms: 500,
            last_move_time_ms: None,
            game_over: false,
            score: 0,
        }
    }

    fn move_snake(&mut self) {
        let delta = match self.direction {
            Direction::Left => Cell(-1, 0),
            Direction::Right => Cell(1, 0),
            Direction::Up => Cell(0, -1),
            Direction::Down => Cell(0, 1),
        };
        let new_cell = *self.snake.front().unwrap() + delta;

        self.game_over = new_cell.0 < 0 ||
            new_cell.0 >= self.width ||
            new_cell.1 < 0 ||
            new_cell.1 >= self.height ||
            self.snake.contains(&new_cell);

        if new_cell == self.apple {
            self.move_delay_ms = (self.move_delay_ms as f32 * 0.9) as u64;
            self.score += 1;
        } else {
            self.snake.pop_back();
        }
        self.snake.push_front(new_cell);
    }

    fn make_apple(width: i32, height: i32, rng: &mut ThreadRng) -> Cell {
        Cell(rng.gen_range(0..width), rng.gen_range(0..height))
    }

    fn new_apple(&mut self) {
        while self.snake.contains(&self.apple) {
            self.apple = Self::make_apple(self.width, self.height, &mut self.rng);
        }
    }

    fn update(&mut self, time_ms: u64) -> Option<State> {
        if self.last_move_time_ms.is_none() {
            self.move_snake();
            self.new_apple();
            self.last_move_time_ms = Some(time_ms);
            return None;
        }

        let mut dt_ms = time_ms - self.last_move_time_ms.unwrap();

        if dt_ms < self.move_delay_ms {
            return None;
        }

        while dt_ms >= self.move_delay_ms {
            self.move_snake();
            self.new_apple();
            dt_ms -= self.move_delay_ms;
        }

        self.last_move_time_ms = Some(time_ms);

        if self.game_over {
            Some(State::GameOver)
        } else {
            None
        }
    }

    fn draw_cell(&self, canvas: &mut WindowCanvas, Cell(x, y): Cell) {
        let cell_width = WIDTH / self.width as u32;
        let cell_height = HEIGHT / self.height as u32;

        canvas.fill_rect(Rect::new(
            x * cell_width as i32,
            y * cell_height as i32,
            cell_width,
            cell_height,
        )).unwrap();
    }

    fn draw(&self, canvas: &mut WindowCanvas, texts: &Texts) {
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();

        canvas.set_draw_color(Color::GREEN);
        for piece in &self.snake {
            self.draw_cell(canvas, *piece);
        }

        canvas.set_draw_color(Color::RED);
        self.draw_cell(canvas, self.apple);

        texts.score.draw(canvas, 0, 0, 0.5);
        texts.draw_number(canvas, self.score, (texts.score.width / 2) as i32, 0, 0.5);
    }
}

enum State {
    Menu,
    Running(RunningGame),
    GameOver,
}

enum Direction { Left, Right, Up, Down }

struct Game {
    state: State,

    scale: f32,
    start: Rect,
    quit: Rect,
}

impl Game {
    fn new(texts: &Texts) -> Self {
        let scale = 0.75;
        let width = (WIDTH / 2) as i32;

        let start_h = scale_by(texts.start.height, scale);
        let start_w = scale_by(texts.start.width, scale);
        let quit_h = scale_by(texts.quit.height, scale);
        let quit_w = scale_by(texts.quit.width, scale);

        let y = (HEIGHT / 2) as i32;
        let start_y = y - (start_h / 2) as i32 - (quit_h / 2) as i32;
        let quit_y = start_y + start_h as i32;

        Game {
            state: State::Menu,
            scale,
            start: Rect::new(width - (start_w / 2) as i32, start_y as i32, start_w, start_h),
            quit: Rect::new(width - (quit_w / 2) as i32, quit_y as i32, quit_w, quit_h),
        }
    }

    fn input(&mut self, keycode: Keycode) {
        let next_state = match &mut self.state {
            State::Menu => None,
            State::Running(game) => {
                match keycode {
                    Keycode::W => game.direction = Direction::Up,
                    Keycode::S => game.direction = Direction::Down,
                    Keycode::D => game.direction = Direction::Right,
                    Keycode::A => game.direction = Direction::Left,
                    _ => {}
                };
                None
            }
            State::GameOver => match keycode {
                Keycode::R => Some(State::Running(RunningGame::new())),
                _ => None
            }
        };

        if next_state.is_some() {
            self.state = next_state.unwrap();
        }
    }

    fn click(&mut self, x: i32, y: i32) {
        let next_state = match &self.state {
            State::Menu => {
                if self.start.contains_point((x, y)) {
                    Some(Running(RunningGame::new()))
                } else if self.quit.contains_point((x, y)) {
                    exit(0);
                } else {
                    None
                }
            }
            _ => None,
        };

        if next_state.is_some() {
            self.state = next_state.unwrap();
        }
    }

    fn update(&mut self, time_ms: u64) {
        let next_state = match &mut self.state {
            State::Running(game) => {
                game.update(time_ms)
            }
            _ => None,
        };

        if next_state.is_some() {
            self.state = next_state.unwrap();
        }
    }

    fn draw(&self, canvas: &mut WindowCanvas, texts: &Texts) {
        let width = (WIDTH / 2) as i32;
        let height = (HEIGHT / 2) as i32;

        match &self.state {
            State::Menu => {
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();

                texts.start.draw(canvas, self.start.x(), self.start.y(), self.scale);
                texts.quit.draw(canvas, self.quit.x(), self.quit.y(), self.scale);
            }
            State::Running(game) => game.draw(canvas, texts),
            State::GameOver => {
                texts.game_over.draw_centered(canvas, width, height, 1.);
            }
        }
    }
}

struct Texts<'a> {
    game_over: Text<'a>,
    score: Text<'a>,
    start: Text<'a>,
    quit: Text<'a>,
    digits: [Text<'a>; 10],
}

impl<'a> Texts<'a> {
    fn new<T>(font: &Font, texture_creator: &'a TextureCreator<T>) -> Self {
        let digits = [
            Text::new(font, texture_creator, "0"),
            Text::new(font, texture_creator, "1"),
            Text::new(font, texture_creator, "2"),
            Text::new(font, texture_creator, "3"),
            Text::new(font, texture_creator, "4"),
            Text::new(font, texture_creator, "5"),
            Text::new(font, texture_creator, "6"),
            Text::new(font, texture_creator, "7"),
            Text::new(font, texture_creator, "8"),
            Text::new(font, texture_creator, "9"),
        ];

        Texts {
            game_over: Text::new(font, texture_creator, "game over"),
            score: Text::new(font, texture_creator, "score: "),
            start: Text::new(font, texture_creator, "start"),
            quit: Text::new(font, texture_creator, "quit"),
            digits,
        }
    }

    fn draw_number(&self, canvas: &mut WindowCanvas, mut n: i32, mut x: i32, y: i32, scale: f32) {
        let mut digits = Vec::new();
        loop {
            digits.push(n % 10);
            n /= 10;
            if n == 0 {
                break;
            }
        }

        for &d in digits.iter().rev() {
            let digit = &self.digits[d as usize];
            digit.draw(canvas, x, y, scale);
            x += digit.width as i32;
        }
    }
}

struct Text<'a> {
    texture: Texture<'a>,
    width: u32,
    height: u32,
}

impl<'a> Text<'a> {
    fn new<T>(font: &Font, texture_creator: &'a TextureCreator<T>, text: &str) -> Self {
        let surface = font.render(text).blended(Color::WHITE).unwrap();
        let width = surface.width();
        let height = surface.height();
        let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
        Text { texture, width, height }
    }

    fn draw_centered(&self, canvas: &mut WindowCanvas, x: i32, y: i32, scale: f32) {
        self.draw(canvas, x - (scale_by(self.width, scale) / 2) as i32, y - (scale_by(self.height, scale) / 2) as i32, scale);
    }

    fn draw(&self, canvas: &mut WindowCanvas, x: i32, y: i32, scale: f32) {
        canvas.copy(&self.texture, None, Rect::new(x, y, scale_by(self.width, scale), scale_by(self.height, scale))).unwrap();
    }
}

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("Snake", WIDTH, HEIGHT)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let timer_subsystem = sdl_context.timer().unwrap();
    let mut prev_update_time = timer_subsystem.performance_counter();
    let ttf_context = sdl2::ttf::init().unwrap();
    let font_bytes = include_bytes!("../assets/shanghai/shanghai.ttf");
    let font = ttf_context.load_font_from_rwops(RWops::from_bytes(font_bytes).unwrap(), 64).unwrap();
    let texture_creator = canvas.texture_creator();
    let texts = Texts::new(&font, &texture_creator);
    let mut game = Game::new(&texts);

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode: Some(keycode), .. } => {
                    game.input(keycode);
                }
                Event::MouseButtonDown { x, y, mouse_btn: MouseButton::Left, .. } => {
                    game.click(x, y);
                }
                _ => {}
            }
        }

        let mut delta = timer_subsystem.performance_counter() - prev_update_time;
        let wait_time = timer_subsystem.performance_frequency() / 60;
        if delta < wait_time {
            continue;
        }

        while delta >= wait_time {
            game.update(timer_subsystem.performance_counter() * 1000 / timer_subsystem.performance_frequency());
            delta -= wait_time;
        }

        prev_update_time = timer_subsystem.performance_counter();

        game.draw(&mut canvas, &texts);

        canvas.present();
    }
}
