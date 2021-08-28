#![no_main]
#![no_std]

// set the panic handler
use panic_halt as _;

use core::convert::Infallible;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use generic_array::typenum::{U5, U6};
use hal::gpio::{gpioa, gpiob, Input, Output, PullUp, PushPull};
use hal::prelude::*;
use hal::serial;
use hal::usb;
use hal::{stm32, timers};
use keyberon::action::{k, l, m, Action, Action::*, HoldTapConfig};
use keyberon::debounce::Debouncer;
use keyberon::impl_heterogenous_array;
use keyberon::key_code::KbHidReport;
use keyberon::key_code::KeyCode::*;
use keyberon::layout::{Event, Layout};
use keyberon::matrix::{Matrix, PressedKeys};
use nb::block;
use rtic::app;
use stm32f0xx_hal as hal;
use usb_device::bus::UsbBusAllocator;
use usb_device::class::UsbClass as _;
use usb_device::device::UsbDeviceState;

type UsbClass = keyberon::Class<'static, usb::UsbBusType, ()>;
type UsbDevice = usb_device::device::UsbDevice<'static, usb::UsbBusType>;

trait ResultExt<T> {
    fn get(self) -> T;
}
impl<T> ResultExt<T> for Result<T, Infallible> {
    fn get(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => match e {},
        }
    }
}

pub struct Cols(
    gpioa::PA0<Input<PullUp>>,
    gpioa::PA1<Input<PullUp>>,
    gpioa::PA2<Input<PullUp>>,
    gpioa::PA3<Input<PullUp>>,
    gpioa::PA4<Input<PullUp>>,
    gpioa::PA5<Input<PullUp>>,
);
impl_heterogenous_array! {
    Cols,
    dyn InputPin<Error = Infallible>,
    U6,
    [0, 1, 2, 3, 4, 5]
}

pub struct Rows(
    gpiob::PB0<Output<PushPull>>,
    gpiob::PB1<Output<PushPull>>,
    gpiob::PB2<Output<PushPull>>,
    gpiob::PB10<Output<PushPull>>,
    gpiob::PB11<Output<PushPull>>,
);
impl_heterogenous_array! {
    Rows,
    dyn OutputPin<Error = Infallible>,
    U5,
    [0, 1, 2, 3, 4]
}

const L2_ENTER: Action = HoldTap {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: &l(2),
    tap: &k(Enter),
};
const CTRL_TAB: Action = HoldTap {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::Default,
    hold: &k(LCtrl),
    tap: &k(Tab),
};
const L1_SP: Action = HoldTap {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: &l(1),
    tap: &k(Space),
};
const SFT_BSP: Action = HoldTap {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::Default,
    hold: &k(RShift),
    tap: &k(BSpace),
};

macro_rules! s {
    ($k:ident) => {
        m(&[LShift, $k])
    };
}
macro_rules! c {
    ($k:ident) => {
        m(&[LCtrl, $k])
    };
}

const WORD_LEFT: Action = c!(Left);
const WORD_RIGHT: Action = c!(Right);
const PREV_TAB: Action = c!(PgUp);
const NEXT_TAB: Action = c!(PgDown);

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers = &[
    &[
        // Layer 0: Alphas
        //-----L0----- , -----L1----- , -----L2----- , -----L3----- , -----L4----- , -----L5----- --SPLIT-- , -----R5----- , -----R4----- , -----R3----- , -----R2----- , -----R1----- , -----R0----- ,
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
        &[Trans        , k(Q)         , k(W)         , k(E)         , k(R)         , k(T)                   , k(Y)         , k(U)         , k(I)         , k(O)         , k(P)         , Trans        ],
        &[Trans        , k(A)         , k(S)         , k(D)         , k(F)         , k(G)                   , k(H)         , k(J)         , k(K)         , k(L)         , k(Escape)    , Trans        ],
        &[Trans        , k(RShift)    , k(Z)         , k(X)         , k(C)         , k(V)                   , k(B)         , k(N)         , k(M)         , k(Delete)    , l(4)         , Trans        ],
        &[Trans        , Trans        , k(LAlt)      , CTRL_TAB     , L1_SP        , k(LGui)                , l(3)         , L2_ENTER     , SFT_BSP      , k(RAlt)      , Trans        , Trans        ],
    ], &[
        // Layer 1: Brackets and Navigation keys
        //-----L0----- , -----L1----- , -----L2----- , -----L3----- , -----L4----- , -----L5-----           , -----R5----- , -----R4----- , -----R3----- , -----R2----- , -----R1----- , -----R0----- ,
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
        &[Trans        , k(No)        , k(No)        , s!(LBracket) , s!(RBracket) , k(No)                  , k(PgUp)      , WORD_LEFT    , k(Up)        , WORD_RIGHT   , k(PScreen)   , Trans        ],
        &[Trans        , k(No)        , s!(Comma)    , s!(Kb9)      , s!(Kb0)      , s!(Dot)                , k(Home)      , k(Left)      , k(Down)      , k(Right)     , k(End)       , Trans        ],
        &[Trans        , Trans        , k(No)        , k(LBracket)  , k(RBracket)  , k(No)                  , k(PgDown)    , PREV_TAB     , NEXT_TAB     , k(Insert)    , Trans        , Trans        ],
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
    ], &[
        // Layer 2: Symbols
        //-----L0----- , -----L1----- , -----L2----- , -----L3----- , -----L4----- , -----L5-----           , -----R5----- , -----R4----- , -----R3----- , -----R2----- , -----R1----- , -----R0----- ,
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
        &[Trans        , k(No)        , s!(Grave)    , s!(Equal)    , s!(Minus)    , k(Bslash)              , s!(Quote)    , k(Comma)     , s!(Slash)    , s!(SColon)   , k(No)        , Trans        ],
        &[Trans        , k(No)        , k(Grave)     , k(Equal)     , k(Minus)     , k(Slash)               , k(Quote)     , k(Dot)       , s!(Kb1)      , k(SColon)    , k(No)        , Trans        ],
        &[Trans        , Trans        , s!(Bslash)   , s!(Kb7)      , s!(Kb8)      , s!(Kb6)                , s!(Kb2)      , s!(Kb3)      , s!(Kb4)      , s!(Kb5)      , Trans        , Trans        ],
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
    ], &[
        // Layer 3: Function and Number keys
        //-----L0----- , -----L1----- , -----L2----- , -----L3----- , -----L4----- , -----L5-----           , -----R5----- , -----R4----- , -----R3----- , -----R2----- , -----R1----- , -----R0----- ,
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
        &[Trans        , k(No)        , k(F1)        , k(F2)        , k(F3)        , k(F4)                  , k(Kb0)       , k(Kb1)       , k(Kb2)       , k(Kb3)       , k(No)        , Trans        ],
        &[Trans        , k(No)        , k(F5)        , k(F6)        , k(F7)        , k(F8)                  , k(Dot)       , k(Kb4)       , k(Kb5)       , k(Kb6)       , k(No)        , Trans        ],
        &[Trans        , Trans        , k(F9)        , k(F10)       , k(F11)       , k(F12)                 , k(No)        , k(Kb7)       , k(Kb8)       , k(Kb9)       , Trans        , Trans        ],
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
    ], &[
        // Layer 4: Thumb keys without tap-hold
        //-----L0----- , -----L1----- , -----L2----- , -----L3----- , -----L4----- , -----L5-----           , -----R5----- , -----R4----- , -----R3----- , -----R2----- , -----R1----- , -----R0----- ,
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
        &[Trans        , Trans        , Trans        , Trans        , Trans        , Trans                  , Trans        , Trans        , Trans        , Trans        , Trans        , Trans        ],
        &[Trans        , Trans        , Trans        , k(Tab)       , k(Space)     , Trans                  , Trans        , k(Enter)     , k(BSpace)    , Trans        , Trans        , Trans        ],
    ],
];

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice,
        usb_class: UsbClass,
        matrix: Matrix<Cols, Rows>,
        debouncer: Debouncer<PressedKeys<U5, U6>>,
        layout: Layout,
        timer: timers::Timer<stm32::TIM3>,
        transform: fn(Event) -> Event,
        tx: serial::Tx<hal::pac::USART1>,
        rx: serial::Rx<hal::pac::USART1>,
    }

    #[init]
    fn init(mut c: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        let mut rcc = c
            .device
            .RCC
            .configure()
            .hsi48()
            .enable_crs(c.device.CRS)
            .sysclk(48.mhz())
            .pclk(24.mhz())
            .freeze(&mut c.device.FLASH);

        let gpioa = c.device.GPIOA.split(&mut rcc);
        let gpiob = c.device.GPIOB.split(&mut rcc);

        let pb8 = gpiob.pb8;
        let mut power_led = cortex_m::interrupt::free(move |cs| pb8.into_push_pull_output(cs));
        power_led.set_high().unwrap();

        let usb = usb::Peripheral {
            usb: c.device.USB,
            pin_dm: gpioa.pa11,
            pin_dp: gpioa.pa12,
        };
        *USB_BUS = Some(usb::UsbBusType::new(usb));
        let usb_bus = USB_BUS.as_ref().unwrap();

        let usb_class = keyberon::new_class(usb_bus, ());
        let usb_dev = keyberon::new_device(usb_bus);

        let mut timer = timers::Timer::tim3(c.device.TIM3, 1.khz(), &mut rcc);
        timer.listen(timers::Event::TimeOut);

        let pb6 = gpiob.pb6;
        let is_flipped = cortex_m::interrupt::free(move |cs| pb6.into_pull_up_input(cs))
            .is_low()
            .get();
        let transform: fn(Event) -> Event = if is_flipped {
            |e| e.transform(|i, j| (i, 11 - j))
        } else {
            |e| e
        };

        let pb9 = gpiob.pb9;
        let mut status_led = cortex_m::interrupt::free(move |cs| pb9.into_push_pull_output(cs));
        if is_flipped {
            status_led.set_high()
        } else {
            status_led.set_low()
        }
        .unwrap();

        let (pa9, pa10) = (gpioa.pa9, gpioa.pa10);
        let pins = cortex_m::interrupt::free(move |cs| {
            (pa9.into_alternate_af1(cs), pa10.into_alternate_af1(cs))
        });
        let mut serial = serial::Serial::usart1(c.device.USART1, pins, 38_400.bps(), &mut rcc);
        serial.listen(serial::Event::Rxne);
        let (tx, rx) = serial.split();

        let pa0 = gpioa.pa0;
        let pa1 = gpioa.pa1;
        let pa2 = gpioa.pa2;
        let pa3 = gpioa.pa3;
        let pa4 = gpioa.pa4;
        let pa5 = gpioa.pa5;
        let pb0 = gpiob.pb0;
        let pb1 = gpiob.pb1;
        let pb2 = gpiob.pb2;
        let pb10 = gpiob.pb10;
        let pb11 = gpiob.pb11;
        let matrix = cortex_m::interrupt::free(move |cs| {
            Matrix::new(
                Cols(
                    pa0.into_pull_up_input(cs),
                    pa1.into_pull_up_input(cs),
                    pa2.into_pull_up_input(cs),
                    pa3.into_pull_up_input(cs),
                    pa4.into_pull_up_input(cs),
                    pa5.into_pull_up_input(cs),
                ),
                Rows(
                    pb0.into_push_pull_output(cs),
                    pb1.into_push_pull_output(cs),
                    pb2.into_push_pull_output(cs),
                    pb10.into_push_pull_output(cs),
                    pb11.into_push_pull_output(cs),
                ),
            )
        });

        init::LateResources {
            usb_dev,
            usb_class,
            timer,
            debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5),
            matrix: matrix.get(),
            layout: Layout::new(LAYERS),
            transform,
            tx,
            rx,
        }
    }

    #[task(binds = USART1, priority = 5, spawn = [handle_event], resources = [rx])]
    fn rx(c: rx::Context) {
        static mut BUF: [u8; 4] = [0; 4];

        if let Ok(b) = c.resources.rx.read() {
            BUF.rotate_left(1);
            BUF[3] = b;

            if BUF[3] == b'\n' {
                if let Ok(event) = de(&BUF[..]) {
                    c.spawn.handle_event(Some(event)).unwrap();
                }
            }
        }
    }

    #[task(binds = USB, priority = 4, resources = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        if c.resources.usb_dev.poll(&mut [c.resources.usb_class]) {
            c.resources.usb_class.poll();
        }
    }

    #[task(priority = 3, capacity = 8, resources = [usb_dev, usb_class, layout])]
    fn handle_event(mut c: handle_event::Context, event: Option<Event>) {
        let report: KbHidReport = match event {
            None => {
                c.resources.layout.tick();
                c.resources.layout.keycodes().collect()
            }
            Some(e) => {
                c.resources.layout.event(e);
                return;
            }
        };
        if !c
            .resources
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        if c.resources.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
            return;
        }
        while let Ok(0) = c.resources.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(
        binds = TIM3,
        priority = 2,
        spawn = [handle_event],
        resources = [matrix, debouncer, timer, &transform, tx],
    )]
    fn tick(c: tick::Context) {
        c.resources.timer.wait().ok();

        for event in c
            .resources
            .debouncer
            .events(c.resources.matrix.get().get())
            .map(c.resources.transform)
        {
            for &b in &ser(event) {
                block!(c.resources.tx.write(b)).get();
            }
            c.spawn.handle_event(Some(event)).unwrap();
        }
        c.spawn.handle_event(None).unwrap();
    }

    extern "C" {
        fn CEC_CAN();
    }
};

fn de(bytes: &[u8]) -> Result<Event, ()> {
    match *bytes {
        [b'P', i, j, b'\n'] => Ok(Event::Press(i, j)),
        [b'R', i, j, b'\n'] => Ok(Event::Release(i, j)),
        _ => Err(()),
    }
}
fn ser(e: Event) -> [u8; 4] {
    match e {
        Event::Press(i, j) => [b'P', i, j, b'\n'],
        Event::Release(i, j) => [b'R', i, j, b'\n'],
    }
}
