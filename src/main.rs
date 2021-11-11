// #![deny(unsafe_code)]
#![no_main]
#![no_std]

use core::fmt::Debug;
use cortex_m::interrupt::free;
use cortex_m_rt::entry;
use cortex_m_rt::exception;
use cortex_m_rt::ExceptionFrame;
use cortex_m_semihosting::hprintln;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;
use panic_halt as _;
use stm32f1xx_hal::delay::Delay;
use stm32f1xx_hal::flash::FlashExt;
use stm32f1xx_hal::gpio::gpioa::PA11;
use stm32f1xx_hal::gpio::gpioa::PA12;
use stm32f1xx_hal::gpio::Floating;
use stm32f1xx_hal::gpio::GpioExt;
use stm32f1xx_hal::gpio::Input;
use stm32f1xx_hal::pac;
use stm32f1xx_hal::pac::interrupt;
use stm32f1xx_hal::pac::Interrupt;
use stm32f1xx_hal::pac::USB;
use stm32f1xx_hal::rcc::RccExt;
use stm32f1xx_hal::time::U32Ext;
use stm32f1xx_hal::usb::Peripheral as UsbPeripheral;
use stm32f1xx_hal::usb::UsbBus;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::device::UsbDevice;
use usb_device::device::UsbDeviceBuilder;
use usb_device::device::UsbVidPid;
use usb_device::UsbError;
use usbd_hid::descriptor::MouseReport;
use usbd_hid::descriptor::SerializedDescriptor;
use usbd_hid::hid_class::HIDClass;
use usbd_hid_device::USB_CLASS_HID;

static mut USB_BUS: Option<UsbBusAllocator<UsbBus<UsbPeripheral>>> = None;

static mut USB_HID: Option<HIDClass<UsbBus<UsbPeripheral>>> = None;

static mut USB_DEVICE: Option<UsbDevice<UsbBus<UsbPeripheral>>> = None;

trait Success<T> {
    fn success(self) -> T;
}

impl<T, E> Success<T> for Result<T, E>
where
    E: Debug,
{
    fn success(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                hprintln!("{:?}", err).ok();
                panic!();
            }
        }
    }
}

#[interrupt]
fn USB_LP_CAN_RX0() {
    free(
        |_cs| match (unsafe { USB_DEVICE.as_mut() }, unsafe { USB_HID.as_mut() }) {
            (Some(device), Some(hid)) => {
                device.poll(&mut [hid]);
            }
            _ => {}
        },
    );
}

#[inline]
fn make_usb_bus(
    usb: USB,
    pa11: PA11<Input<Floating>>,
    pa12: PA12<Input<Floating>>,
) -> &'static UsbBusAllocator<UsbBus<UsbPeripheral>> {
    let usb_bus = UsbBus::new(UsbPeripheral {
        usb,
        pin_dm: pa11,
        pin_dp: pa12,
    });

    unsafe { USB_BUS = Some(usb_bus) };
    unsafe { USB_BUS.as_ref().unwrap() }
}

#[inline]
fn mouse_move_report(x: i8, y: i8) -> MouseReport {
    MouseReport {
        buttons: 0,
        x,
        y,
        wheel: 0,
        pan: 0,
    }
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(48.mhz())
        .pclk1(24.mhz())
        .freeze(&mut flash.acr);
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    let mut delay = Delay::new(cp.SYST, clocks);

    // Reset connection.
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low().success();
    delay.delay_ms(100 as u16);

    let usb_bus = make_usb_bus(
        dp.USB,
        gpioa.pa11,
        usb_dp.into_floating_input(&mut gpioa.crh),
    );
    let usb_hid = HIDClass::new(&usb_bus, MouseReport::desc(), 60);
    let usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27da))
        .manufacturer("Snake's Company")
        .product("Twitchy Mouse")
        .device_class(USB_CLASS_HID)
        .build();
    let actions = [
        mouse_move_report(0, -24),
        mouse_move_report(-9, -22),
        mouse_move_report(-17, -17),
        mouse_move_report(-22, -9),
        mouse_move_report(-24, 0),
        mouse_move_report(-22, 9),
        mouse_move_report(-17, 17),
        mouse_move_report(-9, 22),
        mouse_move_report(0, 24),
        mouse_move_report(9, 22),
        mouse_move_report(17, 17),
        mouse_move_report(22, 9),
        mouse_move_report(24, 0),
        mouse_move_report(22, -9),
        mouse_move_report(17, -17),
        mouse_move_report(9, -22),
    ];

    unsafe { USB_HID = Some(usb_hid) };
    unsafe { USB_DEVICE = Some(usb_device) };

    unsafe {
        pac::NVIC::unmask(Interrupt::USB_LP_CAN_RX0);
    }

    led.set_high().success();

    loop {
        led.set_low().success();

        for action in actions {
            loop {
                let result = free(|_cs| match unsafe { USB_HID.as_mut() } {
                    Some(hid) => hid.push_input(&action),
                    _ => panic!("HID not initialized"),
                });

                match result {
                    Ok(_) => break,
                    Err(UsbError::WouldBlock) => continue,
                    Err(err) => {
                        hprintln!("USB error: {:?}", err).ok();

                        break;
                    }
                }
            }
        }

        led.set_high().success();

        delay.delay_ms(10 * 1_000 as u16);
    }
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    hprintln!("Hard fault: {:#?}", ef).ok();

    panic!();
}

#[exception]
fn DefaultHandler(irqn: i16) {
    hprintln!("Unhandled exception (IRQn = {})", irqn).ok();

    panic!();
}
