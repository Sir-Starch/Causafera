use ontopolis_observer_api::ObserverQuery;
use ontopolis_observer_wire::{ConnectRequest, ProtocolHandler, encode_query};
use ontopolis_runtime::Runtime;

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
