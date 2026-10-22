import SwiftUI
import UIKit

/// Keep new SDK symbols out of builds made with an older Swift compiler.
enum LiquidGlassSupport {
    static var isNativeAvailable: Bool {
        #if compiler(>=6.2)
        if #available(iOS 26.0, *) { return true }
        #endif
        return false
    }
}

/// Glass belongs to the control layer. Message bubbles remain opaque content.
private struct AdaptiveGlassSurface<S: Shape>: ViewModifier {
    let shape: S
    let interactive: Bool
    @EnvironmentObject private var appearance: AppearanceSettings
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorSchemeContrast) private var contrast

    private var translucent: Bool {
        appearance.glassControls && !reduceTransparency && contrast != .increased
    }

    @ViewBuilder
    func body(content: Content) -> some View {
        #if compiler(>=6.2)
        if #available(iOS 26.0, *), translucent {
            content.glassEffect(.regular.interactive(interactive && !reduceMotion), in: shape)
        } else {
            fallback(content)
        }
        #else
        fallback(content)
        #endif
    }

    @ViewBuilder
    private func fallback(_ content: Content) -> some View {
        if translucent {
            content.background { shape.fill(.regularMaterial) }
                .overlay { shape.stroke(.primary.opacity(0.12), lineWidth: 0.5).allowsHitTesting(false) }
        } else {
            content.background { shape.fill(Color(uiColor: .secondarySystemGroupedBackground)) }
                .overlay { shape.stroke(Color(uiColor: .separator), lineWidth: 1).allowsHitTesting(false) }
        }
    }
}

extension View {
    func campusGlass<S: Shape>(in shape: S, interactive: Bool = false) -> some View {
        modifier(AdaptiveGlassSurface(shape: shape, interactive: interactive))
    }
}

/// A spacing smaller than the row gap keeps separate controls from merging at rest.
struct GlassControlGroup<Content: View>: View {
    private let content: Content
    init(@ViewBuilder content: () -> Content) { self.content = content() }

    @ViewBuilder
    var body: some View {
        #if compiler(>=6.2)
        if #available(iOS 26.0, *) {
            GlassEffectContainer(spacing: 6) { content }
        } else {
            content
        }
        #else
        content
        #endif
    }
}

struct GlassSendButton: View {
    let enabled: Bool
    let action: () -> Void
    @EnvironmentObject private var appearance: AppearanceSettings
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.colorSchemeContrast) private var contrast

    private var nativeGlass: Bool {
        appearance.glassControls && !reduceTransparency && contrast != .increased
    }

    var body: some View {
        styledButton
            .disabled(!enabled)
            .accessibilityLabel("发送消息")
    }

    @ViewBuilder
    private var styledButton: some View {
        #if compiler(>=6.2)
        if #available(iOS 26.0, *), nativeGlass {
            Button(action: action) {
                Image(systemName: "paperplane.fill").font(.system(size: 20))
                    .frame(minWidth: 24, minHeight: 24)
            }
            .buttonStyle(.glassProminent)
            .buttonBorderShape(.circle)
            .controlSize(.large)
            .tint(appearance.accent)
            .frame(minWidth: 44, minHeight: 44)
        } else {
            solidButton
        }
        #else
        solidButton
        #endif
    }

    private var solidButton: some View {
        Button(action: action) {
            Image(systemName: "paperplane.fill").font(.system(size: 20))
                .foregroundStyle(contrast == .increased ? Color(uiColor: .systemBackground) : .white)
                .frame(width: 44, height: 44)
                .background(contrast == .increased ? Color.primary : appearance.accent, in: Circle())
                .opacity(enabled ? 1 : 0.4)
        }.buttonStyle(.plain)
    }
}

struct LiquidGlassPreview: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("下课后，图书馆见。")
                .font(.subheadline).padding(12)
                .background(Color(uiColor: .secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 16))
            GlassControlGroup {
                HStack(spacing: 8) {
                    Image(systemName: "text.bubble").frame(width: 44, height: 44)
                        .campusGlass(in: Circle())
                    Text("消息").foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
                        .padding(.horizontal, 14).campusGlass(in: Capsule())
                    GlassSendButton(enabled: true, action: {})
                }
            }
        }
        .padding(14)
        .background { TelegramWallpaper() }
        .clipShape(RoundedRectangle(cornerRadius: 24))
        .allowsHitTesting(false)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("输入栏外观预览")
    }
}
