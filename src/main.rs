// SPDX-FileCopyrightText: 2024 Derek Sauer
//
// SPDX-License-Identifier: GPL-3.0-only
//
#![feature(impl_trait_in_assoc_type)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_variables))]
#![no_main]
#![no_std]

use trouble_host::prelude::*;
use {defmt_rtt as _, panic_probe as _};

embassy_nrf::bind_interrupts!(struct Irqs {
    RNG => embassy_nrf::rng::InterruptHandler<embassy_nrf::peripherals::RNG>;
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

#[gatt_server]
struct GattServer {
    battery_service: BatteryService,
}

#[gatt_service(uuid = service::BATTERY)]
struct BatteryService {
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "level", read, value = "Battery Level")]
    #[characteristic(uuid = characteristic::BATTERY_LEVEL, read, notify, value = 10)]
    level:  u8,
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100000", write, read, notify)]
    status: bool,
}

/// Entry point.
#[embassy_executor::main]
async fn main(task_spawner: embassy_executor::Spawner) {
    defmt::info!("Device is starting up.");

    let peripherals = init_peripherals();

    let mpsl_p = nrf_sdc::mpsl::Peripherals::new(
        peripherals.RTC0,
        peripherals.TIMER0,
        peripherals.TEMP,
        peripherals.PPI_CH19,
        peripherals.PPI_CH30,
        peripherals.PPI_CH31,
    );

    let lfclk_cfg = nrf_sdc::mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source:                  nrf_sdc::mpsl::raw::MPSL_CLOCK_LF_SRC_XTAL as u8,
        rc_ctiv:                 0 as u8,
        rc_temp_ctiv:            0 as u8,
        accuracy_ppm:            50 as u16,
        skip_wait_lfclk_started: nrf_sdc::mpsl::raw::MPSL_DEFAULT_SKIP_WAIT_LFCLK_STARTED != 0,
    };

    static MPSL: static_cell::StaticCell<nrf_mpsl::MultiprotocolServiceLayer> =
        static_cell::StaticCell::new();

    let mpsl =
        MPSL.init(nrf_sdc::mpsl::MultiprotocolServiceLayer::new(mpsl_p, Irqs, lfclk_cfg).unwrap());
    task_spawner.must_spawn(mpsl_task(&*mpsl));

    let sdc_p = nrf_sdc::Peripherals::new(
        peripherals.PPI_CH17,
        peripherals.PPI_CH18,
        peripherals.PPI_CH20,
        peripherals.PPI_CH21,
        peripherals.PPI_CH22,
        peripherals.PPI_CH23,
        peripherals.PPI_CH24,
        peripherals.PPI_CH25,
        peripherals.PPI_CH26,
        peripherals.PPI_CH27,
        peripherals.PPI_CH28,
        peripherals.PPI_CH29,
    );

    let mut rng = embassy_nrf::rng::Rng::new(peripherals.RNG, Irqs);

    let mut sdc_mem = nrf_sdc::Mem::<1488>::new();

    let sdc = nrf_sdc::Builder::new()
        .expect("SDC new builder failed.")
        .support_adv()
        .expect("SDC support advertising failed.")
        .support_peripheral()
        .expect("SDC support peripheral mode failed.")
        .peripheral_count(1)
        .expect("SDC number of peripherals failed.")
        .buffer_cfg(27, 27, 3, 3)
        .expect("SDC buffer config failed.")
        .build(sdc_p, &mut rng, mpsl, &mut sdc_mem)
        .unwrap();

    let ficr = embassy_nrf::pac::FICR;
    let device_address_0 = ficr.deviceaddr(0).read().to_le_bytes();
    let device_address_1 = ficr.deviceaddr(1).read().to_le_bytes();
    let device_address: [u8; 6] = [
        device_address_0[0],
        device_address_0[1],
        device_address_0[2],
        device_address_0[3],
        device_address_1[0],
        device_address_1[1],
    ];
    let ble_address = Address::random(device_address);
    defmt::info!("Our BLE address: {:?}", ble_address);

    let mut resources: HostResources<1, 2, 27> = HostResources::new();
    let stack = trouble_host::new(sdc, &mut resources).set_random_address(ble_address);
    let Host {
        mut peripheral,
        runner,
        ..
    } = stack.build();

    stack.log_status(true);

    let server = GattServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name:       "Lookpoint",
        appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
    }))
    .unwrap();

    let _ = embassy_futures::join::join(ble_task(runner), async {
        loop {
            match advertise("Lookpoint Tracker", &mut peripheral, &server).await {
                Ok(conn) => {
                    let task_a = gatt_events_task(&server, &conn);
                    let task_b = custom_task(&server, &conn, &stack);

                    embassy_futures::select::select(task_a, task_b).await;
                }
                Err(err) => {
                    let err = defmt::Debug2Format(&err);
                    panic!("[adv] error: {:?}", err);
                }
            }
        }
    })
    .await;
}

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static nrf_mpsl::MultiprotocolServiceLayer<'static>) -> ! {
    defmt::info!("Spawning MPSL.");
    mpsl.run().await
}

async fn ble_task<C: Controller>(mut runner: Runner<'_, C>) {
    loop {
        if let Err(e) = runner.run().await {
            let e = defmt::Debug2Format(&e);
            panic!("[ble_task] error: {:?}", e);
        }
    }
}

async fn advertise<'a, 'b, C: Controller>(
    name: &'a str,
    peripheral: &mut Peripheral<'a, C>,
    server: &'b GattServer<'_>,
) -> Result<GattConnection<'a, 'b>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x0f, 0x18]]),
            AdStructure::CompleteLocalName(name.as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data:  &advertiser_data[..],
                scan_data: &[],
            },
        )
        .await?;
    defmt::info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    defmt::info!("[adv] connection established");
    Ok(conn)
}

async fn gatt_events_task(
    server: &GattServer<'_>,
    conn: &GattConnection<'_, '_>,
) -> Result<(), Error> {
    let level = server.battery_service.level;
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                defmt::info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => match event {
                Ok(event) => {
                    match &event {
                        GattEvent::Read(event) => {
                            if event.handle() == level.handle {
                                let value = server.get(&level);
                                defmt::info!(
                                    "[gatt] Read Event to Level Characteristic: {:?}",
                                    value
                                );
                            }
                        }
                        GattEvent::Write(event) => {
                            if event.handle() == level.handle {
                                defmt::info!(
                                    "[gatt] Write Event to Level Characteristic: {:?}",
                                    event.data()
                                );
                            }
                        }
                    }

                    // This step is also performed at drop(), but writing it explicitly is necessary
                    // in order to ensure reply is sent.
                    match event.accept() {
                        Ok(reply) => {
                            reply.send().await;
                        }
                        Err(e) => defmt::warn!("[gatt] error sending response: {:?}", e),
                    }
                }
                Err(e) => defmt::warn!("[gatt] error processing event: {:?}", e),
            },
            _ => {}
        }
    }
    defmt::info!("[gatt] task finished");
    Ok(())
}

async fn custom_task<C: Controller>(
    server: &GattServer<'_>,
    conn: &GattConnection<'_, '_>,
    stack: &Stack<'_, C>,
) {
    let mut tick: u8 = 0;
    let level = server.battery_service.level;
    loop {
        tick = tick.wrapping_add(1);
        defmt::info!("[custom_task] notifying connection of tick {}", tick);
        if level.notify(conn, &tick).await.is_err() {
            defmt::info!("[custom_task] error notifying connection");
            break;
        };
        // read RSSI (Received Signal Strength Indicator) of the connection.
        if let Ok(rssi) = conn.raw().rssi(stack).await {
            defmt::info!("[custom_task] RSSI: {:?}", rssi);
        } else {
            defmt::info!("[custom_task] error getting RSSI");
            break;
        };
        embassy_time::Timer::after_secs(2).await;
    }
}

/// Initialize the MCU, its peripherals, and interrupts.
fn init_peripherals() -> embassy_nrf::Peripherals {
    use embassy_nrf::config;

    let mut nrf_config = config::Config::default();

    // Our board has an external 32Mhz oscillator.
    nrf_config.hfclk_source = config::HfclkSource::ExternalXtal;
    nrf_config.lfclk_source = config::LfclkSource::ExternalXtal;
    nrf_config.dcdc.reg0 = true;
    nrf_config.dcdc.reg0_voltage = None;
    nrf_config.dcdc.reg1 = true;

    defmt::info!("Microcontroller initialized.");

    embassy_nrf::init(nrf_config)
}
