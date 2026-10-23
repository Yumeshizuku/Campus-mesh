import Foundation
import CoreBluetooth

/// Foreground-only BLE adapter shared wire format with Android:
/// 4-byte big-endian length + encrypted Frame JSON, in <=20 byte GATT chunks.
final class BLEMesh: NSObject, CBCentralManagerDelegate, CBPeripheralDelegate, CBPeripheralManagerDelegate {
    static let service = CBUUID(string:"B8554A10-6A9A-4C18-AE32-32A3BC1D0010")
    static let data = CBUUID(string:"B8554A11-6A9A-4C18-AE32-32A3BC1D0010")
    final class Channel {
        var rx = Data()
        var tx = Data()
        var offset = 0
        var cursor = 0
        var last = Date()
        func receive(_ bytes:Data, ingest:(Data)->Void) {
            if Date().timeIntervalSince(last)>30 {rx.removeAll()};last=Date()
            guard rx.count+bytes.count<=65540 else {rx.removeAll();return}
            rx.append(bytes)
            guard rx.count>=4 else {return}
            let n=rx.prefix(4).reduce(0){($0<<8)|Int($1)}
            guard (1...65536).contains(n) else {rx.removeAll();return}
            if rx.count==n+4 {ingest(Data(rx.dropFirst(4)));rx.removeAll()}
            else if rx.count>n+4 {rx.removeAll()}
        }
        func prepare(_ frames:[[String:Any]]) {
            guard offset>=tx.count,!frames.isEmpty else {return}
            let frame=frames[cursor % frames.count];cursor=(cursor+1)%frames.count
            guard let bytes=try? JSONSerialization.data(withJSONObject:frame),bytes.count<=65536 else {return}
            let n=UInt32(bytes.count);tx=Data([UInt8((n>>24)&255),UInt8((n>>16)&255),UInt8((n>>8)&255),UInt8(n&255)]);tx.append(bytes);offset=0
        }
        var chunk:Data {Data(tx[offset..<min(offset+20,tx.count)])}
    }
    var call:(([String:Any])->Any?)?
    var onStatus:((String)->Void)?
    private var central:CBCentralManager!
    private var peripheral:CBPeripheralManager!
    private var dataCharacteristic:CBMutableCharacteristic?
    private var devices=[UUID:CBPeripheral]()
    private var remoteCharacteristics=[UUID:CBCharacteristic]()
    private var ready=Set<UUID>()
    private var clientChannels=[UUID:Channel]()
    private var subscribers=[UUID:CBCentral]()
    private var serverChannels=[UUID:Channel]()
    var syncInterval:()->TimeInterval = {5}
    private var lastSync=Date.distantPast
    private var timer:Timer?
    private var running=false
    func start() {
        guard !running else {return};running=true
        central=CBCentralManager(delegate:self,queue:.main)
        peripheral=CBPeripheralManager(delegate:self,queue:.main)
        timer=Timer.scheduledTimer(withTimeInterval:1,repeats:true){[weak self] _ in
            guard let self=self,Date().timeIntervalSince(self.lastSync)>=self.syncInterval() else{return}
            self.lastSync=Date();self.tick()
        }
        onStatus?("BLE 正在启动")
    }
    func stop(){running=false;timer?.invalidate();timer=nil;central?.stopScan();peripheral?.stopAdvertising();peripheral?.removeAllServices();for p in devices.values{central.cancelPeripheralConnection(p)};devices.removeAll();remoteCharacteristics.removeAll();ready.removeAll();clientChannels.removeAll();subscribers.removeAll();serverChannels.removeAll();onStatus?("BLE 已停止")}
    private func status(){onStatus?("BLE · \(ready.count+subscribers.count) 条 GATT 链路")}
    func centralManagerDidUpdateState(_ manager:CBCentralManager){if running && manager.state == .poweredOn{manager.scanForPeripherals(withServices:[Self.service],options:[CBCentralManagerScanOptionAllowDuplicatesKey:true])}else{onStatus?("BLE 不可用：检查蓝牙与权限")}}
    func centralManager(_ manager:CBCentralManager,didDiscover device:CBPeripheral,advertisementData:[String:Any],rssi RSSI:NSNumber){guard running,devices.count<4,devices[device.identifier]==nil else{return};devices[device.identifier]=device;clientChannels[device.identifier]=Channel();device.delegate=self;manager.connect(device)}
    func centralManager(_ manager:CBCentralManager,didConnect device:CBPeripheral){device.discoverServices([Self.service])}
    func centralManager(_ manager:CBCentralManager,didFailToConnect device:CBPeripheral,error:Error?){remove(device)}
    func centralManager(_ manager:CBCentralManager,didDisconnectPeripheral device:CBPeripheral,error:Error?){remove(device)}
    private func remove(_ device:CBPeripheral){devices.removeValue(forKey:device.identifier);remoteCharacteristics.removeValue(forKey:device.identifier);ready.remove(device.identifier);clientChannels.removeValue(forKey:device.identifier);status()}
    func peripheral(_ device:CBPeripheral,didDiscoverServices error:Error?){guard error==nil,let service=device.services?.first(where:{$0.uuid==Self.service}) else{central.cancelPeripheralConnection(device);return};device.discoverCharacteristics([Self.data],for:service)}
    func peripheral(_ device:CBPeripheral,didDiscoverCharacteristicsFor service:CBService,error:Error?){guard error==nil,let characteristic=service.characteristics?.first(where:{$0.uuid==Self.data}) else{central.cancelPeripheralConnection(device);return};remoteCharacteristics[device.identifier]=characteristic;device.setNotifyValue(true,for:characteristic)}
    func peripheral(_ device:CBPeripheral,didUpdateNotificationStateFor characteristic:CBCharacteristic,error:Error?){if error==nil && characteristic.isNotifying{ready.insert(device.identifier);status()}else{central.cancelPeripheralConnection(device)}}
    private func ingest(_ bytes:Data){guard let value=try? JSONSerialization.jsonObject(with:bytes) else{return};_ = call?(["op":"ingest","frame":value])}
    func peripheral(_ device:CBPeripheral,didUpdateValueFor characteristic:CBCharacteristic,error:Error?){guard error==nil,characteristic.uuid==Self.data,let value=characteristic.value else{return};clientChannels[device.identifier]?.receive(value,ingest:ingest)}
    func peripheral(_ device:CBPeripheral,didWriteValueFor characteristic:CBCharacteristic,error:Error?){guard error==nil,let channel=clientChannels[device.identifier] else{central.cancelPeripheralConnection(device);return};channel.offset+=min(20,channel.tx.count-channel.offset);pumpClient(device,channel)}
    private func pumpClient(_ device:CBPeripheral,_ channel:Channel){guard channel.offset<channel.tx.count,let characteristic=remoteCharacteristics[device.identifier] else{return};device.writeValue(channel.chunk,for:characteristic,type:.withResponse)}
    func peripheralManagerDidUpdateState(_ manager:CBPeripheralManager){guard running,manager.state == .poweredOn else{onStatus?("BLE 广播不可用：检查蓝牙权限");return}
        let service=CBMutableService(type:Self.service,primary:true)
        let c=CBMutableCharacteristic(type:Self.data,properties:[.write,.notify],value:nil,permissions:[.writeable]);dataCharacteristic=c;service.characteristics=[c];manager.add(service)
    }
    func peripheralManager(_ manager:CBPeripheralManager,didAdd service:CBService,error:Error?){guard running,error==nil else{return};manager.startAdvertising([CBAdvertisementDataServiceUUIDsKey:[Self.service]])}
    func peripheralManager(_ manager:CBPeripheralManager,central:CBCentral,didSubscribeTo characteristic:CBCharacteristic){guard subscribers.count<4 else{return};subscribers[central.identifier]=central;serverChannels[central.identifier]=Channel();status()}
    func peripheralManager(_ manager:CBPeripheralManager,central:CBCentral,didUnsubscribeFrom characteristic:CBCharacteristic){subscribers.removeValue(forKey:central.identifier);serverChannels.removeValue(forKey:central.identifier);status()}
    func peripheralManager(_ manager:CBPeripheralManager,didReceiveWrite requests:[CBATTRequest]){
        for request in requests {
            guard request.characteristic.uuid==Self.data,request.offset==0,let value=request.value,let channel=serverChannels[request.central.identifier] else{manager.respond(to:request,withResult:.requestNotSupported);continue}
            channel.receive(value,ingest:ingest);manager.respond(to:request,withResult:.success)
        }
    }
    private func pumpServer(_ subscriber:CBCentral,_ channel:Channel){guard let c=dataCharacteristic else{return};while channel.offset<channel.tx.count{if !peripheral.updateValue(channel.chunk,for:c,onSubscribedCentrals:[subscriber]){return};channel.offset+=min(20,channel.tx.count-channel.offset)}}
    func peripheralManagerIsReady(toUpdateSubscribers manager:CBPeripheralManager){for (id,c) in serverChannels{if let subscriber=subscribers[id]{pumpServer(subscriber,c)}}}
    private func tick(){guard running,let frames=call?(["op":"outgoing","limit":1]) as? [[String:Any]] else{return}
        for id in ready {if let p=devices[id],let c=clientChannels[id],c.offset>=c.tx.count{c.prepare(frames);pumpClient(p,c)}}
        for (id,c) in serverChannels {if let subscriber=subscribers[id],c.offset>=c.tx.count{c.prepare(frames);pumpServer(subscriber,c)}}
    }
}
