import SwiftUI
import AVFoundation
struct QRScanner: UIViewControllerRepresentable {
    var onCode:(String)->Void
    func makeUIViewController(context:Context)->ScannerController {let controller=ScannerController();controller.onCode=onCode;return controller}
    func updateUIViewController(_ uiViewController:ScannerController,context:Context) {}
}
final class ScannerController: UIViewController,AVCaptureMetadataOutputObjectsDelegate {
    var onCode:((String)->Void)?
    private let session=AVCaptureSession()
    private let queue=DispatchQueue(label:"org.campusmesh.qr")
    private var preview:AVCaptureVideoPreviewLayer?
    private var delivered=false
    override func viewDidLoad(){super.viewDidLoad();view.backgroundColor = .black
        AVCaptureDevice.requestAccess(for:.video){[weak self] granted in guard let self=self else{return}; if granted {self.queue.async{self.configure()}} else {DispatchQueue.main.async{self.showError("请在系统设置中允许相机，或使用公钥文本添加好友。")}}}
    }
    private func configure(){guard let device=AVCaptureDevice.default(for:.video),let input=try? AVCaptureDeviceInput(device:device),session.canAddInput(input) else {DispatchQueue.main.async{self.showError("相机不可用")};return};session.addInput(input)
        let output=AVCaptureMetadataOutput();guard session.canAddOutput(output) else{return};session.addOutput(output);output.setMetadataObjectsDelegate(self,queue:.main);output.metadataObjectTypes=[.qr]
        DispatchQueue.main.async{let layer=AVCaptureVideoPreviewLayer(session:self.session);layer.videoGravity = .resizeAspectFill;layer.frame=self.view.bounds;self.view.layer.addSublayer(layer);self.preview=layer}
        session.startRunning()
    }
    private func showError(_ text:String){let label=UILabel(frame:view.bounds.insetBy(dx:24,dy:24));label.text=text;label.textColor = .white;label.numberOfLines=0;view.addSubview(label)}
    override func viewDidLayoutSubviews(){super.viewDidLayoutSubviews();preview?.frame=view.bounds}
    override func viewDidDisappear(_ animated:Bool){super.viewDidDisappear(animated);queue.async{self.session.stopRunning()}}
    func metadataOutput(_ output:AVCaptureMetadataOutput,didOutput metadataObjects:[AVMetadataObject],from connection:AVCaptureConnection){guard !delivered,let code=metadataObjects.first as? AVMetadataMachineReadableCodeObject,let value=code.stringValue else{return};delivered=true;onCode?(value)}
}
