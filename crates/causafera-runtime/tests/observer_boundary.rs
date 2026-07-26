use causafera_observer_api::ObserverQuery;
use causafera_observer_wire::{ConnectRequest, ProtocolHandler, encode_query};
use causafera_runtime::Runtime;

/// Every locale the observer offers, as the front end sends them on connect.
const OBSERVER_LOCALES: [&str; 5] = ["en-US", "ru-RU", "zh-Hans", "de-DE", "es-ES"];

#[test]
fn locale_and_observer_activity_do_not_change_authoritative_digests() {
    let mut english_runtime = Runtime::from_seed(0x0b5e_7e12).unwrap();
    let mut russian_runtime = Runtime::from_seed(0x0b5e_7e12).unwrap();

    for tick in 0..24 {
        let english = english_runtime.tick().unwrap();
        let russian = russian_runtime.tick().unwrap();

        let mut english_handler = ProtocolHandler::new(english.time);
        english_handler.set_runtime_snapshot(&english.observer_snapshot());
        english_handler
            .negotiate(&ConnectRequest {
                supported_versions: vec![1],
                locale: "en-US".into(),
            })
            .unwrap();
        english_handler
            .handle_query(&encode_query(&ObserverQuery::runtime_summary(tick)))
            .unwrap();

        if tick % 5 == 0 {
            let mut russian_handler = ProtocolHandler::new(russian.time);
            russian_handler.set_runtime_snapshot(&russian.observer_snapshot());
            russian_handler
                .negotiate(&ConnectRequest {
                    supported_versions: vec![1],
                    locale: "ru-RU".into(),
                })
                .unwrap();
        }

        assert_eq!(english.physical_state_digest, russian.physical_state_digest);
        assert_eq!(english.history_digest, russian.history_digest);
    }
}

/// INV-007 across the whole locale set, not merely across the two the observer opened with.
///
/// One runtime per locale, advanced identically, each observed through a handler negotiated in
/// its own locale. A locale that reached authoritative state — through the handler, through a
/// query, through anything — would show up as a digest divergence here.
#[test]
fn every_supported_locale_preserves_authoritative_digests() {
    // Given: one runtime per supported locale, all from the same seed.
    let mut runtimes: Vec<(&str, Runtime)> = OBSERVER_LOCALES
        .iter()
        .map(|locale| (*locale, Runtime::from_seed(0x10ca_1e00).unwrap()))
        .collect();

    for tick in 0..16 {
        // When: every runtime advances one tick and is observed in its own locale.
        let mut digests = Vec::new();
        for (locale, runtime) in runtimes.iter_mut() {
            let outcome = runtime.tick().unwrap();

            let mut handler = ProtocolHandler::new(outcome.time);
            handler.set_runtime_snapshot(&outcome.observer_snapshot());
            handler
                .negotiate(&ConnectRequest {
                    supported_versions: vec![1],
                    locale: (*locale).into(),
                })
                .unwrap();
            handler
                .handle_query(&encode_query(&ObserverQuery::runtime_summary(tick)))
                .unwrap();

            digests.push((
                *locale,
                outcome.physical_state_digest,
                outcome.history_digest,
            ));
        }

        // Then: the digests are identical across locales at every tick.
        let (first_locale, first_physical, first_history) = digests[0];
        for (locale, physical, history) in digests.iter().skip(1) {
            assert_eq!(
                first_physical, *physical,
                "physical digest diverged between {first_locale} and {locale} at tick {tick}"
            );
            assert_eq!(
                first_history, *history,
                "history digest diverged between {first_locale} and {locale} at tick {tick}"
            );
        }
    }
}
