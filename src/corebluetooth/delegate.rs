// btleplug Source Code File
//
// Copyright 2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.
//
// Some portions of this file are taken and/or modified from blurmac
// (https://github.com/servo/devices), using a BSD 3-Clause license under the
// following copyright:
//
// Copyright (c) 2017 Akos Kiss.
//
// Licensed under the BSD 3-Clause License
// <LICENSE.md or https://opensource.org/licenses/BSD-3-Clause>.
// This file may not be copied, modified, or distributed except
// according to those terms.

use std::error::Error;
use std::sync::{Once};

use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Protocol, Sel};

use super::{
    framework::{nil, cb, ns},
    utils::{NO_PERIPHERAL_FOUND, cbx, nsx, wait},
};

pub mod bm {
    use super::*;

    // BlurMacDelegate : CBCentralManagerDelegate, CBPeripheralDelegate

    const DELEGATE_PERIPHERALS_IVAR: &'static str = "_peripherals";

    fn delegate_class() -> &'static Class {
        trace!("delegate_class");
        static REGISTER_DELEGATE_CLASS: Once = Once::new();
        let mut decl = ClassDecl::new("BlurMacDelegate", Class::get("NSObject").unwrap()).unwrap();

        REGISTER_DELEGATE_CLASS.call_once(|| {
            decl.add_protocol(Protocol::get("CBCentralManagerDelegate").unwrap());

            decl.add_ivar::<*mut Object>(DELEGATE_PERIPHERALS_IVAR); /* NSMutableDictionary<NSString*, BlurMacPeripheralData*>* */

            unsafe {
                decl.add_method(sel!(init), delegate_init as extern fn(&mut Object, Sel) -> *mut Object);
                decl.add_method(sel!(centralManagerDidUpdateState:), delegate_centralmanagerdidupdatestate as extern fn(&mut Object, Sel, *mut Object));
                // decl.add_method(sel!(centralManager:willRestoreState:), delegate_centralmanager_willrestorestate as extern fn(&mut Object, Sel, *mut Object, *mut Object));
                decl.add_method(sel!(centralManager:didConnectPeripheral:), delegate_centralmanager_didconnectperipheral as extern fn(&mut Object, Sel, *mut Object, *mut Object));
                decl.add_method(sel!(centralManager:didDisconnectPeripheral:error:), delegate_centralmanager_diddisconnectperipheral_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
                // decl.add_method(sel!(centralManager:didFailToConnectPeripheral:error:), delegate_centralmanager_didfailtoconnectperipheral_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
                decl.add_method(sel!(centralManager:didDiscoverPeripheral:advertisementData:RSSI:), delegate_centralmanager_diddiscoverperipheral_advertisementdata_rssi as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object, *mut Object));

                decl.add_method(sel!(peripheral:didDiscoverServices:), delegate_peripheral_diddiscoverservices as extern fn(&mut Object, Sel, *mut Object, *mut Object));
                decl.add_method(sel!(peripheral:didDiscoverIncludedServicesForService:error:), delegate_peripheral_diddiscoverincludedservicesforservice_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
                decl.add_method(sel!(peripheral:didDiscoverCharacteristicsForService:error:), delegate_peripheral_diddiscovercharacteristicsforservice_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
                decl.add_method(sel!(peripheral:didUpdateValueForCharacteristic:error:), delegate_peripheral_didupdatevalueforcharacteristic_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
                decl.add_method(sel!(peripheral:didWriteValueForCharacteristic:error:), delegate_peripheral_didwritevalueforcharacteristic_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
                decl.add_method(sel!(peripheral:didReadRSSI:error:), delegate_peripheral_didreadrssi_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            }

            decl.register();
        });

        Class::get("BlurMacDelegate").unwrap()
    }

    extern fn delegate_init(delegate: &mut Object, _cmd: Sel) -> *mut Object {
        trace!("delegate_init");
        unsafe {
            delegate.set_ivar::<*mut Object>(DELEGATE_PERIPHERALS_IVAR, ns::mutabledictionary());
        }
        delegate
    }

    extern fn delegate_centralmanagerdidupdatestate(_delegate: &mut Object, _cmd: Sel, _central: *mut Object) {
        trace!("delegate_centralmanagerdidupdatestate");
        // NOTE: this is a no-op but kept because it is a required method of the protocol
    }

    // extern fn delegate_centralmanager_willrestorestate(_delegate: &mut Object, _cmd: Sel, _central: *mut Object, _dict: *mut Object) {
    //     trace!("delegate_centralmanager_willrestorestate");
    // }

    extern fn delegate_centralmanager_didconnectperipheral(delegate: &mut Object, _cmd: Sel, _central: *mut Object, peripheral: *mut Object) {
        trace!("delegate_centralmanager_didconnectperipheral {}", cbx::peripheral_debug(peripheral));
        cb::peripheral_setdelegate(peripheral, delegate);
        cb::peripheral_discoverservices(peripheral);
    }

    extern fn delegate_centralmanager_diddisconnectperipheral_error(delegate: &mut Object, _cmd: Sel, _central: *mut Object, peripheral: *mut Object, _error: *mut Object) {
        trace!("delegate_centralmanager_diddisconnectperipheral_error {}", cbx::peripheral_debug(peripheral));
        ns::mutabledictionary_removeobjectforkey(delegate_peripherals(delegate), ns::uuid_uuidstring(cb::peer_identifier(peripheral)));
    }

    // extern fn delegate_centralmanager_didfailtoconnectperipheral_error(_delegate: &mut Object, _cmd: Sel, _central: *mut Object, _peripheral: *mut Object, _error: *mut Object) {
    //     trace!("delegate_centralmanager_didfailtoconnectperipheral_error");
    // }

    extern fn delegate_centralmanager_diddiscoverperipheral_advertisementdata_rssi(delegate: &mut Object, _cmd: Sel, _central: *mut Object, peripheral: *mut Object, adv_data: *mut Object, rssi: *mut Object) {
        trace!("delegate_centralmanager_diddiscoverperipheral_advertisementdata_rssi {}", cbx::peripheral_debug(peripheral));
        let peripherals = delegate_peripherals(delegate);
        let uuid_nsstring = ns::uuid_uuidstring(cb::peer_identifier(peripheral));
        let mut data = ns::dictionary_objectforkey(peripherals, uuid_nsstring);
        if data == nil {
            data = ns::mutabledictionary();
            ns::mutabledictionary_setobject_forkey(peripherals, data, uuid_nsstring);
        }

        ns::mutabledictionary_setobject_forkey(data, ns::object_copy(peripheral), nsx::string_from_str(PERIPHERALDATA_PERIPHERALKEY));

        ns::mutabledictionary_setobject_forkey(data, rssi, nsx::string_from_str(PERIPHERALDATA_RSSIKEY));

        let cbuuids_nsarray = ns::dictionary_objectforkey(adv_data, unsafe { cb::ADVERTISEMENTDATASERVICEUUIDSKEY });
        if cbuuids_nsarray != nil {
            ns::mutabledictionary_setobject_forkey(data, cbuuids_nsarray, nsx::string_from_str(PERIPHERALDATA_UUIDSKEY));
        }

        if ns::dictionary_objectforkey(data, nsx::string_from_str(PERIPHERALDATA_EVENTSKEY)) == nil {
            ns::mutabledictionary_setobject_forkey(data, ns::mutabledictionary(), nsx::string_from_str(PERIPHERALDATA_EVENTSKEY));
        }
    }

    extern fn delegate_peripheral_diddiscoverservices(delegate: &mut Object, _cmd: Sel, peripheral: *mut Object, error: *mut Object) {
        trace!("delegate_peripheral_diddiscoverservices {} {}", cbx::peripheral_debug(peripheral), if error != nil {"error"} else {""});
        if error == nil {
            let services = cb::peripheral_services(peripheral);
            for i in 0..ns::array_count(services) {
                let s = ns::array_objectatindex(services, i);
                cb::peripheral_discovercharacteristicsforservice(peripheral, s);
                cb::peripheral_discoverincludedservicesforservice(peripheral, s);
            }

            // Notify BluetoothDevice::get_gatt_services that discovery was successful.
            match bmx::peripheralevents(delegate, peripheral) {
                Ok(events) => ns::mutabledictionary_setobject_forkey(events, wait::now(), nsx::string_from_str(PERIPHERALEVENT_SERVICESDISCOVEREDKEY)),
                Err(_) => {},
            }
        }
    }

    extern fn delegate_peripheral_diddiscoverincludedservicesforservice_error(delegate: &mut Object, _cmd: Sel, peripheral: *mut Object, service: *mut Object, error: *mut Object) {
        trace!("delegate_peripheral_diddiscoverincludedservicesforservice_error {} {} {}", cbx::peripheral_debug(peripheral), cbx::service_debug(service), if error != nil {"error"} else {""});
        if error == nil {
            let includes = cb::service_includedservices(service);
            for i in 0..ns::array_count(includes) {
                let s = ns::array_objectatindex(includes, i);
                cb::peripheral_discovercharacteristicsforservice(peripheral, s);
            }

            // Notify BluetoothGATTService::get_includes that discovery was successful.
            match bmx::peripheralevents(delegate, peripheral) {
                Ok(events) => ns::mutabledictionary_setobject_forkey(events, wait::now(), bmx::includedservicesdiscoveredkey(service)),
                Err(_) => {},
            }
        }
    }

    extern fn delegate_peripheral_diddiscovercharacteristicsforservice_error(delegate: &mut Object, _cmd: Sel, peripheral: *mut Object, service: *mut Object, error: *mut Object) {
        trace!("delegate_peripheral_diddiscovercharacteristicsforservice_error {} {} {}", cbx::peripheral_debug(peripheral), cbx::service_debug(service), if error != nil {"error"} else {""});
        if error == nil {
            let chars = cb::service_characteristics(service);
            for i in 0..ns::array_count(chars) {
                let c = ns::array_objectatindex(chars, i);
                cb::peripheral_discoverdescriptorsforcharacteristic(peripheral, c);
            }

            // Notify BluetoothGATTService::get_gatt_characteristics that discovery was successful.
            match bmx::peripheralevents(delegate, peripheral) {
                Ok(events) => ns::mutabledictionary_setobject_forkey(events, wait::now(), bmx::characteristicsdiscoveredkey(service)),
                Err(_) => {},
            }
        }
    }

    extern fn delegate_peripheral_didupdatevalueforcharacteristic_error(delegate: &mut Object, _cmd: Sel, peripheral: *mut Object, characteristic: *mut Object, error: *mut Object) {
        trace!("delegate_peripheral_didupdatevalueforcharacteristic_error {} {} {}", cbx::peripheral_debug(peripheral), cbx::characteristic_debug(characteristic), if error != nil {"error"} else {""});
        if error == nil {
            // Notify BluetoothGATTCharacteristic::read_value that read was successful.
            match bmx::peripheralevents(delegate, peripheral) {
                Ok(events) => ns::mutabledictionary_setobject_forkey(events, wait::now(), bmx::valueupdatedkey(characteristic)),
                Err(_) => {},
            }
        }
    }

    extern fn delegate_peripheral_didwritevalueforcharacteristic_error(delegate: &mut Object, _cmd: Sel, peripheral: *mut Object, characteristic: *mut Object, error: *mut Object) {
        trace!("delegate_peripheral_didwritevalueforcharacteristic_error {} {} {}", cbx::peripheral_debug(peripheral), cbx::characteristic_debug(characteristic), if error != nil {"error"} else {""});
        if error == nil {
            // Notify BluetoothGATTCharacteristic::write_value that write was successful.
            match bmx::peripheralevents(delegate, peripheral) {
                Ok(events) => ns::mutabledictionary_setobject_forkey(events, wait::now(), bmx::valuewrittenkey(characteristic)),
                Err(_) => {},
            }
        }
    }

    // extern fn delegate_peripheral_didupdatenotificationstateforcharacteristic_error(_delegate: &mut Object, _cmd: Sel, _peripheral: *mut Object, _characteristic: *mut Object, _error: *mut Object) {
    //     trace!("delegate_peripheral_didupdatenotificationstateforcharacteristic_error");
    //     // TODO: this is where notifications should be handled...
    // }

    // extern fn delegate_peripheral_diddiscoverdescriptorsforcharacteristic_error(_delegate: &mut Object, _cmd: Sel, _peripheral: *mut Object, _characteristic: *mut Object, _error: *mut Object) {
    //     trace!("delegate_peripheral_diddiscoverdescriptorsforcharacteristic_error");
    // }

    // extern fn delegate_peripheral_didupdatevaluefordescriptor(_delegate: &mut Object, _cmd: Sel, _peripheral: *mut Object, _descriptor: *mut Object, _error: *mut Object) {
    //     trace!("delegate_peripheral_didupdatevaluefordescriptor");
    // }

    // extern fn delegate_peripheral_didwritevaluefordescriptor_error(_delegate: &mut Object, _cmd: Sel, _peripheral: *mut Object, _descriptor: *mut Object, _error: *mut Object) {
    //     trace!("delegate_peripheral_didwritevaluefordescriptor_error");
    // }

    extern fn delegate_peripheral_didreadrssi_error(delegate: &mut Object, _cmd: Sel, peripheral: *mut Object, rssi: *mut Object, error: *mut Object) {
        trace!("delegate_peripheral_didreadrssi_error {}", cbx::peripheral_debug(peripheral));
        if error == nil {
            let peripherals = delegate_peripherals(delegate);
            let uuid_nsstring = ns::uuid_uuidstring(cb::peer_identifier(peripheral));
            let data = ns::dictionary_objectforkey(peripherals, uuid_nsstring);
            if data != nil {
                ns::mutabledictionary_setobject_forkey(data, rssi, nsx::string_from_str(PERIPHERALDATA_RSSIKEY));
            }
        }
    }

    pub fn delegate() -> *mut Object {
        unsafe {
            let mut delegate: *mut Object = msg_send![delegate_class(), alloc];
            delegate = msg_send![delegate, init];
            delegate
        }
    }

    pub fn delegate_peripherals(delegate: *mut Object) -> *mut Object {
        unsafe {
            let peripherals: *mut Object = *(&mut *delegate).get_ivar::<*mut Object>(DELEGATE_PERIPHERALS_IVAR);
            peripherals
        }
    }

    // "BlurMacPeripheralData" = NSMutableDictionary<NSString*, id>

    pub const PERIPHERALDATA_PERIPHERALKEY: &'static str = "peripheral";
    pub const PERIPHERALDATA_RSSIKEY: &'static str = "rssi";
    pub const PERIPHERALDATA_UUIDSKEY: &'static str = "uuids";
    pub const PERIPHERALDATA_EVENTSKEY: &'static str = "events";

    pub const PERIPHERALEVENT_SERVICESDISCOVEREDKEY: &'static str = "services";
    pub const PERIPHERALEVENT_INCLUDEDSERVICESDISCOVEREDKEYSUFFIX: &'static str = ":includes";
    pub const PERIPHERALEVENT_CHARACTERISTICSDISCOVEREDKEYSUFFIX: &'static str = ":characteristics";
    pub const PERIPHERALEVENT_VALUEUPDATEDKEYSUFFIX: &'static str = ":updated";
    pub const PERIPHERALEVENT_VALUEWRITTENKEYSUFFIX: &'static str = ":written";
}

pub mod bmx {
    use super::*;

    pub fn peripheraldata(delegate: *mut Object, peripheral: *mut Object) -> Result<*mut Object, Box<dyn Error>> {
        let peripherals = bm::delegate_peripherals(delegate);
        let data = ns::dictionary_objectforkey(peripherals, ns::uuid_uuidstring(cb::peer_identifier(peripheral)));
        if data == nil {
            warn!("peripheraldata -> NOT FOUND");
            return Err(Box::from(NO_PERIPHERAL_FOUND));
        }
        Ok(data)
    }

    pub fn peripheralevents(delegate: *mut Object, peripheral: *mut Object) -> Result<*mut Object, Box<dyn Error>> {
        let data = peripheraldata(delegate, peripheral)?;
        Ok(ns::dictionary_objectforkey(data, nsx::string_from_str(bm::PERIPHERALDATA_EVENTSKEY)))
    }

    pub fn includedservicesdiscoveredkey(service: *mut Object) -> *mut Object {
        suffixedkey(service, bm::PERIPHERALEVENT_INCLUDEDSERVICESDISCOVEREDKEYSUFFIX)
    }

    pub fn characteristicsdiscoveredkey(service: *mut Object) -> *mut Object {
        suffixedkey(service, bm::PERIPHERALEVENT_CHARACTERISTICSDISCOVEREDKEYSUFFIX)
    }

    pub fn valueupdatedkey(characteristic: *mut Object) -> *mut Object {
        suffixedkey(characteristic, bm::PERIPHERALEVENT_VALUEUPDATEDKEYSUFFIX)
    }

    pub fn valuewrittenkey(characteristic: *mut Object) -> *mut Object {
        suffixedkey(characteristic, bm::PERIPHERALEVENT_VALUEWRITTENKEYSUFFIX)
    }

    fn suffixedkey(attribute: *mut Object, suffix: &str) -> *mut Object {
        let key = format!("{}{}", cbx::uuid_to_canonical_uuid_string(cb::attribute_uuid(attribute)), suffix);
        nsx::string_from_str(key.as_str())
    }
}
