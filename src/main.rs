use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{
    collections::VecDeque,
    io::{self},
    sync::mpsc::{self, TryRecvError},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DirectionSnake {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Point {
    x: u16,
    y: u16,
}

struct Game {
    snake: VecDeque<Point>,
    dir: DirectionSnake,
    food: Point,
    width: u16,
    height: u16,
    game_over: bool,
    score: usize,
}

impl Game {
    fn new(width: u16, height: u16) -> Self {
        let mut snake = VecDeque::new();
        let start = Point { x: width / 2, y: height / 2 };
        snake.push_back(start);
        let food = Point { x: width / 3, y: height / 3 };
        Self {
            snake,
            dir: DirectionSnake::Right,
            food,
            width,
            height,
            game_over: false,
            score: 0,
        }
    }

    fn step(&mut self) {
        if self.game_over { return; }
        let mut new_head = *self.snake.front().unwrap();
        match self.dir {
            DirectionSnake::Up => {
                if new_head.y == 0 {
                    self.game_over = true;
                    return;
                }
                new_head.y -= 1;
            }
            DirectionSnake::Down => {
                new_head.y += 1;
                if new_head.y >= self.height {
                    self.game_over = true;
                    return;
                }
            }
            DirectionSnake::Left => {
                if new_head.x == 0 {
                    self.game_over = true;
                    return;
                }
                new_head.x -= 1;
            }
            DirectionSnake::Right => {
                new_head.x += 1;
                if new_head.x >= self.width {
                    self.game_over = true;
                    return;
                }
            }
        }
        if self.snake.contains(&new_head) {
            self.game_over = true;
            return;
        }
        self.snake.push_front(new_head);
        if new_head == self.food {
            self.score += 1;
            self.spawn_food();
        } else {
            self.snake.pop_back();
        }
    }

    fn spawn_food(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();
        loop {
            let x = rng.random_range(0..self.width);
            let y = rng.random_range(0..self.height);
            let p = Point { x, y };
            if !self.snake.contains(&p) {
                self.food = p;
                break;
            }
        }
    }

    fn change_dir(&mut self, dir: DirectionSnake) {
        // Если длина змейки 1 — разрешаем любое направление
        if self.snake.len() == 1 {
            self.dir = dir;
            return;
        }
        // Не даём развернуться на 180
        match (self.dir, dir) {
            (DirectionSnake::Up, DirectionSnake::Down) => {}
            (DirectionSnake::Down, DirectionSnake::Up) => {}
            (DirectionSnake::Left, DirectionSnake::Right) => {}
            (DirectionSnake::Right, DirectionSnake::Left) => {}
            _ => self.dir = dir,
        }
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        loop {
            if event::poll(Duration::from_millis(10)).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    tx.send(key).unwrap();
                }
            }
        }
    });

    // let width = 30;
    // let height = 20;
    // let mut game = Game::new(width, height);
    // Вместо фиксированных размеров, инициализируем после первого draw
    let mut game: Option<Game> = None;
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(120);

    let mut paused = false;

    loop {
        terminal.draw(|f| {
            let size = f.area();
            // Размеры поля = размер терминала минус рамка (по 2 по x и y)
            let width = size.width.saturating_sub(2);
            let height = size.height.saturating_sub(2); // исправлено: только рамка

            // Инициализация игры если ещё не была
            if game.is_none() {
                game.replace(Game::new(width, height));
            }
            let game = game.as_mut().unwrap();

            // Если размеры изменились (resize терминала) — обновляем размеры поля, сохраняем прогресс, ставим на паузу
            if game.width != width || game.height != height {
                // Проверяем, помещается ли змейка и еда в новые размеры
                let snake_fits = game.snake.iter().all(|p| p.x < width && p.y < height);
                let food_fits = game.food.x < width && game.food.y < height;
                game.width = width;
                game.height = height;
                if !snake_fits || !food_fits {
                    game.game_over = true;
                }
                paused = true;
            }

            // Рисуем рамку поля
            let block = Block::default().borders(Borders::ALL).title("Змейка (ESC - пауза, пробел - рестарт)");
            f.render_widget(block, size);

            // Игровое поле (без границ, только змейка и еда)
            let mut rows = Vec::new();
            for y in 0..game.height {
                let mut line = Vec::new();
                for x in 0..game.width {
                    let p = Point { x, y };
                    if game.snake.front().unwrap() == &p {
                        line.push(Span::styled("O", Style::default().fg(Color::Green)));
                    } else if game.snake.contains(&p) {
                        line.push(Span::styled("o", Style::default().fg(Color::Green)));
                    } else if game.food == p {
                        line.push(Span::styled("*", Style::default().fg(Color::Red)));
                    } else {
                        line.push(Span::raw(" "));
                    }
                }
                rows.push(Line::from(line));
            }
            // Смещаем игровое поле на +1 по x и +1 по y, чтобы оно было внутр�� рамки
            let area = ratatui::layout::Rect {
                x: size.x + 1,
                y: size.y + 1,
                width: game.width,
                height: game.height,
            };
            let para = Paragraph::new(rows);
            f.render_widget(para, area);

            // Счёт внизу по центру (ровно под рамкой)
            let score_str = format!("Счёт: {}", game.score);
            let score_span = Span::styled(&score_str, Style::default().fg(Color::Yellow));
            let score_line = Line::from(score_span);
            let score_para = Paragraph::new(score_line);
            let score_x = size.x + (size.width / 2).saturating_sub((score_str.len() / 2) as u16);
            let score_y = size.y + game.height + 1; // теперь ровно под рамкой
            f.render_widget(
                score_para,
                ratatui::layout::Rect {
                    x: score_x,
                    y: score_y,
                    width: score_str.len() as u16,
                    height: 1,
                },
            );

            if game.game_over {
                let over = Paragraph::new(vec![
                    Line::from(Span::styled("Игра окончена!", Style::default().fg(Color::Red))),
                    Line::from(Span::styled("Пробел - рестарт", Style::default().fg(Color::White))),
                    Line::from(Span::styled("ESC - выход", Style::default().fg(Color::White))),
                ]);
                let area = ratatui::layout::Rect {
                    x: size.x + (size.width / 2) - 11,
                    y: size.y + game.height / 2,
                    width: 22,
                    height: 4,
                };
                f.render_widget(over, area);
            } else if paused {
                let pause = Paragraph::new(vec![
                    Line::from(Span::styled("Пауза", Style::default().fg(Color::Yellow))),
                    Line::from(Span::styled("ESC - продолжить", Style::default().fg(Color::White))),
                ]);
                let area = ratatui::layout::Rect {
                    x: size.x + (size.width / 2) - 9,
                    y: size.y + game.height / 2,
                    width: 18,
                    height: 3,
                };
                f.render_widget(pause, area);
            }
        })?;

        // Обработка ввода
        match rx.try_recv() {
            Ok(KeyEvent { code, modifiers: _, kind, .. }) => {
                // Обрабатываем только отпускание клавиши
                if kind != KeyEventKind::Release {
                    // Игнорируем все события кроме отпускания
                    continue;
                }
                let game = game.as_mut().unwrap();
                if game.game_over {
                    match code {
                        KeyCode::Char(' ') => {
                            // Пересоздаём игру с текущими размерами
                            *game = Game::new(game.width, game.height);
                            paused = false;
                            last_tick = Instant::now();
                        }
                        KeyCode::Esc => break,
                        _ => {}
                    }
                } else if paused {
                    match code {
                        KeyCode::Esc => paused = false, // ESC снимает паузу
                        _ => {}
                    }
                } else {
                    match code {
                        KeyCode::Esc => paused = true, // ESC ставит на паузу только если не game_over и не paused
                        KeyCode::Up => game.change_dir(DirectionSnake::Up),
                        KeyCode::Down => game.change_dir(DirectionSnake::Down),
                        KeyCode::Left => game.change_dir(DirectionSnake::Left),
                        KeyCode::Right => game.change_dir(DirectionSnake::Right),
                        _ => {}
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(_) => break,
        }

        // step только если игра инициализирована
        if let Some(game) = game.as_mut() {
            if !game.game_over && !paused && last_tick.elapsed() >= tick_rate {
                game.step();
                last_tick = Instant::now();
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
