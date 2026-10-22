import SwiftUI

/// A familiar messenger layout; Campus Mesh retains its own identity and protocol.
struct TelegramAvatar: View {
    let name:String
    let key:String
    var size:CGFloat=54
    private var color:Color {
        let palette:[Color]=[Color(red:0.37,green:0.62,blue:0.80),Color(red:0.48,green:0.49,blue:0.78),Color(red:0.32,green:0.67,blue:0.58),Color(red:0.81,green:0.55,blue:0.37),Color(red:0.73,green:0.45,blue:0.59)]
        let hash=key.unicodeScalars.reduce(0){($0+Int($1.value))%palette.count}
        return palette[hash]
    }
    var body:some View {
        Text(name.first.map{String($0).uppercased()} ?? "?")
            .font(.system(size:size*0.42,weight:.medium)).foregroundStyle(.white)
            .frame(width:size,height:size).background(LinearGradient(colors:[color.opacity(0.75),color],startPoint:.top,endPoint:.bottom),in:Circle())
            .accessibilityHidden(true)
    }
}

struct TelegramWallpaper:View {
    @Environment(\.colorScheme) private var colorScheme
    var body:some View {
        Color(red:colorScheme == .dark ? 0.055 : 0.86,green:colorScheme == .dark ? 0.10 : 0.90,blue:colorScheme == .dark ? 0.14 : 0.87)
            .overlay {
                Canvas {context,size in
                    var path=Path()
                    for x in stride(from:CGFloat(-60),through:size.width+60,by:80) {
                        for y in stride(from:CGFloat(-60),through:size.height+60,by:80) {
                            path.addRoundedRect(in:CGRect(x:x,y:y,width:23,height:23),cornerSize:CGSize(width:8,height:8))
                        }
                    }
                    context.stroke(path,with:.color(.primary.opacity(0.025)),lineWidth:1)
                }
            }
    }
}

struct TelegramBubbleShape:Shape {
    let incoming:Bool
    func path(in rect:CGRect)->Path {
        let r:CGFloat=16,tail:CGFloat=4
        var path=Path()
        path.move(to:CGPoint(x:r,y:0))
        path.addLine(to:CGPoint(x:rect.maxX-r,y:0))
        path.addQuadCurve(to:CGPoint(x:rect.maxX,y:r),control:CGPoint(x:rect.maxX,y:0))
        path.addLine(to:CGPoint(x:rect.maxX,y:rect.maxY-(incoming ? r:tail)))
        path.addQuadCurve(to:CGPoint(x:rect.maxX-(incoming ? r:tail),y:rect.maxY),control:CGPoint(x:rect.maxX,y:rect.maxY))
        path.addLine(to:CGPoint(x:incoming ? tail:r,y:rect.maxY))
        path.addQuadCurve(to:CGPoint(x:0,y:rect.maxY-(incoming ? tail:r)),control:CGPoint(x:0,y:rect.maxY))
        path.addLine(to:CGPoint(x:0,y:r))
        path.addQuadCurve(to:CGPoint(x:r,y:0),control:.zero);path.closeSubpath();return path
    }
}
