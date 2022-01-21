#![no_std]
#![no_main]

use adafruit_macropad::hal;
use cortex_m_rt::entry;
use embedded_hal::digital::v2::*;
use embedded_time::rate::Hertz;
use hal::pac;
use hal::Clock;
use log::*;
use packed_struct::prelude::*;
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_hid_devices::device::keyboard::{new_boot_keyboard, BootKeyboardReport, KeyboardLeds};
use usbd_hid_devices::page::Keyboard;
use usbd_hid_devices_example_rp2040::*;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    //display
    // These are implicitly used by the spi driver if they are in the correct mode
    let _spi_sclk = pins.gpio26.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_mosi = pins.gpio27.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_miso = pins.gpio28.into_mode::<hal::gpio::FunctionSpi>();
    let spi = hal::spi::Spi::<_, _, 8>::new(pac.SPI1);

    // Display control pins
    let oled_dc = pins.gpio24.into_push_pull_output();
    let oled_cs = pins.gpio22.into_push_pull_output();
    let mut oled_reset = pins.gpio23.into_push_pull_output();

    oled_reset.set_high().ok(); //disable screen reset

    // Exchange the uninitialised SPI driver for an initialised one
    let oled_spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        Hertz::new(16_000_000u32),
        &embedded_hal::spi::MODE_0,
    );

    let button = pins.gpio0.into_pull_up_input();

    init_display(oled_spi, oled_dc.into(), oled_cs.into());
    check_for_persisted_panic(&button);
    info!("Starting up...");

    //USB
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut keyboard = new_boot_keyboard(&usb_bus).build().unwrap();

    //https://pid.codes
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .manufacturer("DLKJ")
        .product("Keyboard")
        .serial_number("TEST")
        .device_class(3) // HID - from: https://www.usb.org/defined-class-codes
        .composite_with_iads()
        .supports_remote_wakeup(true)
        .build();

    //GPIO pins
    let mut led_pin = pins.gpio13.into_push_pull_output();

    let in0 = pins.gpio1.into_pull_up_input();
    let in1 = pins.gpio2.into_pull_up_input();
    let in2 = pins.gpio3.into_pull_up_input();
    let in3 = pins.gpio4.into_pull_up_input();
    let in4 = pins.gpio5.into_pull_up_input();
    let in5 = pins.gpio6.into_pull_up_input();
    let in6 = pins.gpio7.into_pull_up_input();
    let in7 = pins.gpio8.into_pull_up_input();
    let in8 = pins.gpio9.into_pull_up_input();
    let in9 = pins.gpio10.into_pull_up_input();
    let in10 = pins.gpio11.into_pull_up_input();
    let in11 = pins.gpio12.into_pull_up_input();

    led_pin.set_low().ok();

    loop {
        if button.is_low().unwrap() {
            hal::rom_data::reset_to_usb_boot(0x1 << 13, 0x0);
        }
        if usb_dev.poll(&mut [&mut keyboard]) {
            let keys = [
                if in0.is_low().unwrap() {
                    Keyboard::KeypadNumLockAndClear
                } else {
                    Keyboard::NoEventIndicated
                }, //Numlock
                if in1.is_low().unwrap() {
                    Keyboard::UpArrow
                } else {
                    Keyboard::NoEventIndicated
                }, //Up
                if in2.is_low().unwrap() {
                    Keyboard::F12
                } else {
                    Keyboard::NoEventIndicated
                }, //F12
                if in3.is_low().unwrap() {
                    Keyboard::LeftArrow
                } else {
                    Keyboard::NoEventIndicated
                }, //Left
                if in4.is_low().unwrap() {
                    Keyboard::DownArrow
                } else {
                    Keyboard::NoEventIndicated
                }, //Down
                if in5.is_low().unwrap() {
                    Keyboard::RightArrow
                } else {
                    Keyboard::NoEventIndicated
                }, //Right
                if in6.is_low().unwrap() {
                    Keyboard::A
                } else {
                    Keyboard::NoEventIndicated
                }, //A
                if in7.is_low().unwrap() {
                    Keyboard::B
                } else {
                    Keyboard::NoEventIndicated
                }, //B
                if in8.is_low().unwrap() {
                    Keyboard::C
                } else {
                    Keyboard::NoEventIndicated
                }, //C
                if in9.is_low().unwrap() {
                    Keyboard::LeftControl
                } else {
                    Keyboard::NoEventIndicated
                }, //LCtrl
                if in10.is_low().unwrap() {
                    Keyboard::LeftShift
                } else {
                    Keyboard::NoEventIndicated
                }, //LShift
                if in11.is_low().unwrap() {
                    Keyboard::ReturnEnter
                } else {
                    Keyboard::NoEventIndicated
                }, //Enter
            ];

            let data = &mut [0];
            match keyboard.get_interface_mut(0).unwrap().read_report(data) {
                Err(UsbError::WouldBlock) => {
                    //do nothing
                }
                Err(e) => {
                    panic!("Failed to read keyboard report: {:?}", e)
                }
                Ok(_) => {
                    //send scroll lock to the led
                    led_pin
                        .set_state(PinState::from(KeyboardLeds::unpack(data).unwrap().num_lock))
                        .ok();
                }
            }

            match keyboard
                .get_interface_mut(0)
                .unwrap()
                .write_report(&BootKeyboardReport::new(keys).pack().unwrap())
            {
                Err(UsbError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => {
                    panic!("Failed to write keyboard report: {:?}", e)
                }
            };
        }
    }
}
