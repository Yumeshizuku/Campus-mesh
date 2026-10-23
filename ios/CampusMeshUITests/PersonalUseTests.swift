import XCTest
import CoreGraphics

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
        XCTAssertTrue(app.buttons["粘贴二维码文本"].waitForExistence(timeout: 5))
        capture("02-附近")
        app.tabBars.buttons["身份"].tap()
        XCTAssertTrue(app.images["我的好友公钥二维码"].waitForExistence(timeout: 10))
        let deviceID=app.staticTexts["deviceID"]
        XCTAssertTrue(deviceID.waitForExistence(timeout:5))
        let originalID=String(deviceID.label.suffix(64))
        XCTAssertEqual(originalID.count,64)
        XCTAssertTrue(originalID.allSatisfy {"0123456789abcdef".contains($0)})
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
        reveal(compact, in: app)
        let previous = compact.value as? String
        // SwiftUI exposes the full labeled row as the switch accessibility frame.
        // Touch the trailing control, and wait for its accessibility value to update.
        compact.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).tap()
        let didChange = NSPredicate { _, _ in (compact.value as? String) != previous }
        let change = XCTNSPredicateExpectation(predicate: didChange, object: compact)
        XCTAssertEqual(XCTWaiter.wait(for: [change], timeout: 5), .completed)
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
        XCTAssertEqual(String(app.staticTexts["deviceID"].label.suffix(64)),originalID,"Changing the nickname and restarting must preserve the device ID")
        app.tabBars.buttons["自定义"].tap()
        XCTAssertTrue(compact.waitForExistence(timeout: 5))
        XCTAssertEqual(compact.value as? String, changed, "Appearance changes must survive a full app restart")
        XCTAssertFalse(app.alerts["Campus Mesh"].exists)
    }


    func testSocialSettingsAndGroupPersistence() throws {
        continueAfterFailure=false
        executionTimeAllowance=300
        let app=XCUIApplication()
        app.launchArguments=["-AppleLanguages","(zh-Hans)","-AppleLocale","zh_CN"]
        app.launch()
        app.tabBars.buttons["附近"].tap()
        let parameters=app.buttons["详细传输参数"]
        reveal(parameters,in:app);parameters.tap()
        XCTAssertTrue(app.navigationBars["Mesh 详细参数"].waitForExistence(timeout:10))
        let hops=app.textFields["最大跳数"]
        XCTAssertTrue(hops.exists)
        hops.tap()
        let current=hops.value as? String ?? ""
        hops.typeText(String(repeating:XCUIKeyboardKey.delete.rawValue,count:current.count)+"4")
        app.buttons["收起键盘"].tap()
        let save=app.buttons["保存 Mesh 参数"]
        reveal(save,in:app);save.tap()
        XCTAssertTrue(app.staticTexts["参数已保存到本机"].waitForExistence(timeout:5))
        capture("05-Mesh参数")
        app.tabBars.buttons["聊天"].tap()
        app.buttons["新建会话"].tap()
        app.buttons["创建群聊"].tap()
        let name="测试群 "+UUID().uuidString.prefix(6)
        let nameField=app.textFields["例如：自习小组"]
        XCTAssertTrue(nameField.waitForExistence(timeout:5));nameField.tap();nameField.typeText(name)
        app.buttons["保存"].tap()
        XCTAssertTrue(app.staticTexts[name].waitForExistence(timeout:10))
        app.staticTexts[name].tap()
        app.buttons["会话操作"].tap();app.buttons["群聊信息"].tap()
        XCTAssertTrue(app.buttons["生成入群二维码"].waitForExistence(timeout:5))
        app.buttons["生成入群二维码"].tap()
        XCTAssertTrue(app.images["群邀请二维码"].waitForExistence(timeout:5))
        capture("06-群邀请")
        app.terminate();app.launch()
        app.tabBars.buttons["聊天"].tap()
        XCTAssertTrue(app.staticTexts[name].waitForExistence(timeout:10),"Managed groups must survive restart")
        app.tabBars.buttons["附近"].tap()
        reveal(parameters,in:app);parameters.tap()
        XCTAssertEqual(app.textFields["最大跳数"].value as? String,"4","Mesh settings must survive restart")
        XCTAssertFalse(app.alerts["Campus Mesh"].exists)
    }

    func testTrustedFriendRemarkSurvivesRestart() throws {
        continueAfterFailure=false
        executionTimeAllowance=300
        let app=XCUIApplication()
        app.launchArguments=["-AppleLanguages","(zh-Hans)","-AppleLocale","zh_CN"]
        app.launch();app.tabBars.buttons["附近"].tap()
        let paste=app.buttons["粘贴二维码文本"]
        XCTAssertTrue(paste.waitForExistence(timeout:10));paste.tap()
        let text=app.textViews.firstMatch
        XCTAssertTrue(text.waitForExistence(timeout:5));text.tap()
        // Public RFC test-vector keys exercise the supported legacy QR import.
        text.typeText("{\"nickname\":\"测试同学\",\"sign\":\"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a\",\"exchange\":\"8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a\"}")
        app.buttons["添加"].tap()
        app.tabBars.buttons["聊天"].tap()
        XCTAssertTrue(app.staticTexts["测试同学"].waitForExistence(timeout:10));app.staticTexts["测试同学"].tap()
        app.buttons["会话操作"].tap();app.buttons["好友信息与备注"].tap()
        let remark=app.textFields["仅自己可见的备注"]
        XCTAssertTrue(remark.waitForExistence(timeout:5));remark.tap();remark.typeText("实验搭档")
        app.buttons["保存备注"].tap()
        XCTAssertFalse(app.alerts["Campus Mesh"].exists)
        app.terminate();app.launch();app.tabBars.buttons["聊天"].tap()
        XCTAssertTrue(app.staticTexts["实验搭档"].waitForExistence(timeout:10),"Friend remarks must survive restart")
        capture("07-好友备注")
    }

    private func reveal(_ element: XCUIElement, in app: XCUIApplication) {
        for _ in 0..<5 {
            if element.exists && element.isHittable && element.frame.maxY < app.tabBars.firstMatch.frame.minY { return }
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
