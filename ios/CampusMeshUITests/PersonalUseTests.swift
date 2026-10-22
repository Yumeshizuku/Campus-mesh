import XCTest

final class PersonalUseTests: XCTestCase {
    func testPersonalUseAndPersistence() throws {
        continueAfterFailure = false
        // Cold first launch on hosted Intel Macs can take over 90 seconds.
        executionTimeAllowance = 300
        let app = XCUIApplication()
        // Do not override ui.tab through launch arguments: the argument domain has
        // precedence over persisted preferences and would pin TabView to one page.
        app.launchArguments = ["-AppleLanguages", "(zh-Hans)", "-AppleLocale", "zh_CN"]
        addUIInterruptionMonitor(withDescription: "System permission") { alert in
            for title in ["允许", "Allow", "好", "OK", "不允许", "Don’t Allow", "Don't Allow"] {
                let button = alert.buttons[title]
                if button.exists { button.tap(); return true }
            }
            return false
        }
        app.launch()
        // Trigger permission handling before checking app content behind the alert.
        app.tap()
        app.tabBars.buttons["聊天"].tap()
        let ready = app.staticTexts["本机已就绪 · 0 位好友"]
        XCTAssertTrue(ready.waitForExistence(timeout: 30), "The real Rust core must start successfully")
        capture("01-聊天")

        app.tabBars.buttons["附近"].tap()
        XCTAssertTrue(app.buttons["粘贴公钥文本"].waitForExistence(timeout: 5))
        capture("02-附近")
        app.tabBars.buttons["身份"].tap()
        XCTAssertTrue(app.images["我的好友公钥二维码"].waitForExistence(timeout: 10))
        app.buttons["编辑"].tap()
        let editor = app.alerts["修改本机昵称"]
        let field = editor.textFields.firstMatch
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        field.tap()
        let current = field.value as? String ?? ""
        field.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: current.count))
        let nickname = "Alpha " + UUID().uuidString.prefix(8)
        field.typeText(nickname)
        editor.buttons["保存"].tap()
        XCTAssertTrue(app.staticTexts[nickname].waitForExistence(timeout: 5))
        capture("03-身份")

        let pause = app.buttons["暂停通信"]
        reveal(pause, in: app)
        pause.tap()
        XCTAssertTrue(app.buttons["恢复通信"].waitForExistence(timeout: 5))
        app.tabBars.buttons["聊天"].tap()
        XCTAssertTrue(app.staticTexts["通信已暂停"].waitForExistence(timeout: 5))
        app.tabBars.buttons["身份"].tap()
        let resume = app.buttons["恢复通信"]
        reveal(resume, in: app)
        resume.tap()
        app.tabBars.buttons["聊天"].tap()
        XCTAssertTrue(ready.waitForExistence(timeout: 10))

        app.tabBars.buttons["自定义"].tap()
        let compact = app.switches["紧凑气泡"]
        XCTAssertTrue(compact.waitForExistence(timeout: 5))
        let previous = compact.value as? String
        compact.tap()
        let changed = compact.value as? String
        XCTAssertNotEqual(changed, previous)
        capture("04-自定义")
        app.terminate()
        app.launch()
        app.tabBars.buttons["聊天"].tap()
        XCTAssertTrue(ready.waitForExistence(timeout: 30))
        app.tabBars.buttons["身份"].tap()
        XCTAssertTrue(app.staticTexts[nickname].waitForExistence(timeout: 10), "Nickname must survive a full app restart")
        XCTAssertTrue(app.images["我的好友公钥二维码"].exists)
        app.tabBars.buttons["自定义"].tap()
        XCTAssertTrue(compact.waitForExistence(timeout: 5))
        XCTAssertEqual(compact.value as? String, changed, "Appearance changes must survive a full app restart")
        XCTAssertFalse(app.alerts["Campus Mesh"].exists)
    }

    private func reveal(_ element: XCUIElement, in app: XCUIApplication) {
        for _ in 0..<5 {
            if element.exists && element.isHittable { return }
            app.swipeUp()
        }
        XCTAssertTrue(element.exists && element.isHittable)
    }

    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
