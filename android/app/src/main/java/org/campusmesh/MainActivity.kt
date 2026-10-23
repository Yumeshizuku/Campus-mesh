package org.campusmesh

import android.Manifest
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.content.res.ColorStateList
import android.content.res.Configuration
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.*
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.*
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.core.graphics.ColorUtils
import androidx.core.view.GravityCompat
import androidx.drawerlayout.widget.DrawerLayout
import com.google.android.material.bottomnavigation.BottomNavigationView
import com.google.android.material.button.MaterialButton
import com.google.android.material.card.MaterialCardView
import com.google.android.material.floatingactionbutton.FloatingActionButton
import com.google.android.material.switchmaterial.SwitchMaterial
import com.google.zxing.integration.android.IntentIntegrator
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import org.json.JSONArray
import org.json.JSONObject

/** Native chat UI; device identities, contacts and managed groups live in the Rust core. */
class MainActivity : AppCompatActivity() {
    private val prefs by lazy { getSharedPreferences("mesh", 0) }
    private val ui by lazy { getSharedPreferences("appearance", 0) }
    private val timer = Handler(Looper.getMainLooper())
    private var data = JSONObject()
    private var screen = "chat"
    private var conversation = ""
    private var search = ""
    private var chatOpen = false
    private var threadQuery = ""
    private var threadFilter = "all"
    private var threadList: LinearLayout? = null
    private var threadStamp = ""
    private var revealSearch = false
    private val readIds by lazy {
        (ui.getStringSet("readIds", emptySet()) ?: emptySet()).toMutableSet()
    }
    private lateinit var status: TextView
    private lateinit var body: LinearLayout
    private var drawer: DrawerLayout? = null
    private var messages: LinearLayout? = null
    private var messageScroll: ScrollView? = null
    private var composer: EditText? = null
    private var sendButton: MaterialButton? = null
    private var chooser: MaterialButton? = null
    private var peerSummary: TextView? = null
    private var lastRender = ""
    private var lastContacts = ""
    private var applyingDraft = false
    private var active = false
    private val dark
        get() =
            resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
                Configuration.UI_MODE_NIGHT_YES

    private val ink
        get() = Color.parseColor(if (dark) "#E8F0F2" else "#182C34")

    private val secondary
        get() = Color.parseColor(if (dark) "#A5B9C1" else "#526A75")

    private val canvas
        get() = Color.parseColor(if (dark) "#17212B" else "#FFFFFF")

    private val surface
        get() = Color.parseColor(if (dark) "#202B36" else "#FFFFFF")

    private val accent
        get(): Int {
            val colors =
                when (ui.getString("accent", "blue")) {
                    "blue" -> "#2782BA" to "#7CBCE8"
                    "violet" -> "#7950A7" to "#D5B7FF"
                    "amber" -> "#96501F" to "#FFC38E"
                    else -> "#246F64" to "#80DCC7"
                }
            return Color.parseColor(if (dark) colors.second else colors.first)
        }

    private val font
        get() = ui.getInt("font", 16).toFloat()

    private val compact
        get() = ui.getBoolean("compact", false)

    private val refresh =
        object : Runnable {
            override fun run() {
                if (active) {
                    refreshState()
                    timer.postDelayed(this, 2000)
                }
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        AppCompatDelegate.setDefaultNightMode(
            when (ui.getString("theme", "system")) {
                "light" -> AppCompatDelegate.MODE_NIGHT_NO
                "dark" -> AppCompatDelegate.MODE_NIGHT_YES
                else -> AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM
            }
        )
        setTheme(
            when (ui.getString("accent", "blue")) {
                "blue" -> R.style.AppThemeBlue
                "violet" -> R.style.AppThemeViolet
                "amber" -> R.style.AppThemeAmber
                "teal" -> R.style.AppTheme
                else -> R.style.AppThemeBlue
            }
        )
        super.onCreate(savedInstanceState)
        screen = savedInstanceState?.getString("screen") ?: ui.getString("tab", "chat")!!
        conversation = ui.getString("conversation", "")!!
        chatOpen = savedInstanceState?.getBoolean("chatOpen") ?: false
        onBackPressedDispatcher.addCallback(
            this,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    if (drawer?.isDrawerOpen(GravityCompat.START) == true) {
                        drawer?.closeDrawer(GravityCompat.START)
                    } else if (screen == "chat" && chatOpen) {
                        chatOpen = false
                        search = ""
                        show("chat")
                    } else if (screen != "chat") {
                        chatOpen = false
                        show("chat")
                    } else finish()
                }
            },
        )
        if (!prefs.contains("nickname")) {
            show(screen)
            val name = input("你的昵称")
            AlertDialog.Builder(this)
                .setTitle("欢迎来到 Campus Mesh")
                .setMessage("建立本机身份，再当面扫码添加同学。")
                .setView(name)
                .setCancelable(false)
                .setPositiveButton("开始使用") { _, _ ->
                    prefs
                        .edit()
                        .putString("nickname", name.text.toString().ifBlank { "同学" }.take(24))
                        .apply()
                    startMesh()
                }
                .show()
        } else startMesh()
    }

    private fun dp(n: Int) = (n * resources.displayMetrics.density).toInt()

    private fun startMesh() {
        startForegroundService(Intent(this, MeshService::class.java))
        show(screen)
        refreshState()
    }

    private fun notice(message: String) {
        Toast.makeText(this, message, Toast.LENGTH_LONG).show()
    }

    private fun request(op: String, change: (JSONObject) -> Unit = {}): JSONObject? =
        try {
            val command = JSONObject().put("op", op)
            change(command)
            Native.request(command).getJSONObject("data")
        } catch (e: Exception) {
            notice(e.message ?: "操作失败")
            null
        }

    private fun label(value: String, size: Float = font, muted: Boolean = false) =
        TextView(this).apply {
            text = value
            textSize = size
            setTextColor(if (muted) secondary else ink)
            setPadding(0, dp(4), 0, dp(4))
        }

    private fun input(hintText: String) =
        EditText(this).apply {
            hint = hintText
            textSize = font
            setTextColor(ink)
            setHintTextColor(secondary)
            setSingleLine()
            minHeight = dp(48)
        }

    private fun watch(edit: EditText, action: (String) -> Unit) {
        edit.addTextChangedListener(
            object : TextWatcher {
                override fun beforeTextChanged(
                    s: CharSequence?,
                    start: Int,
                    count: Int,
                    after: Int,
                ) {}

                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                    action(s.toString())
                }

                override fun afterTextChanged(s: Editable?) {}
            }
        )
    }

    private fun button(title: String, filled: Boolean = false, action: () -> Unit) =
        MaterialButton(this).apply {
            text = title
            isAllCaps = false
            minHeight = dp(48)
            cornerRadius = dp(14)
            textSize = 14f
            setTextColor(if (filled) (if (dark) canvas else Color.WHITE) else accent)
            backgroundTintList = ColorStateList.valueOf(if (filled) accent else surface)
            strokeWidth = if (filled) 0 else dp(1)
            strokeColor = ColorStateList.valueOf(accent)
            setOnClickListener { action() }
        }

    private fun column() = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }

    private fun card(title: String, subtitle: String? = null, build: LinearLayout.() -> Unit) {
        val content = column().apply { setPadding(dp(16), dp(12), dp(16), dp(12)) }
        content.addView(label(title, 18f).apply { setTypeface(null, Typeface.BOLD) })
        if (subtitle != null) content.addView(label(subtitle, 13f, true))
        content.build()
        body.addView(
            MaterialCardView(this).apply {
                radius = dp(20).toFloat()
                cardElevation = 0f
                setCardBackgroundColor(surface)
                addView(content)
            },
            LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(12) },
        )
    }

    private fun show(tab: String) {
        screen = tab
        ui.edit().putString("tab", tab).apply()
        messages = null
        messageScroll = null
        composer = null
        chooser = null
        peerSummary = null
        sendButton = null
        lastRender = ""
        threadList = null
        threadStamp = ""
        val root =
            column().apply {
                setBackgroundColor(canvas)
                fitsSystemWindows = true
            }
        val toolbarColor =
            if (dark) Color.parseColor("#202D38")
            else if (ui.getString("accent", "blue") == "blue") Color.parseColor("#527DA3")
            else accent
        root.setBackgroundColor(toolbarColor)
        window.statusBarColor = toolbarColor
        val header =
            LinearLayout(this).apply {
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(4), dp(4), dp(8), dp(4))
                setBackgroundColor(toolbarColor)
            }
        header.addView(
            icon(
                if (tab == "chat" && chatOpen) R.drawable.mesh_back else R.drawable.mesh_menu,
                if (chatOpen && tab == "chat") "返回会话列表" else "打开导航菜单",
                Color.WHITE,
            ) {
                if (tab == "chat" && chatOpen) {
                    chatOpen = false
                    search = ""
                    show("chat")
                } else drawer?.openDrawer(GravityCompat.START)
            }
        )
        if (tab == "chat" && chatOpen) header.addView(avatar(conversationTitle(), conversation, 36))
        val heading = column().apply { setPadding(dp(12), 0, 0, 0) }
        heading.addView(
            label(
                    if (tab == "chat") {
                        if (chatOpen) conversationTitle() else "Campus Mesh"
                    } else
                        when (tab) {
                            "nearby" -> "附近设备"
                            "identity" -> "我的身份"
                            else -> "设置"
                        },
                    20f,
                )
                .apply {
                    setTextColor(Color.WHITE)
                    setTypeface(null, Typeface.BOLD)
                    maxLines = 1
                    ellipsize = android.text.TextUtils.TruncateAt.END
                }
        )
        status =
            label("正在连接…", 12f).apply {
                setTextColor(Color.parseColor("#DCEAF4"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            }
        heading.addView(status)
        header.addView(heading, LinearLayout.LayoutParams(0, -2, 1f))
        header.addView(
            icon(
                if (tab == "chat" && chatOpen) R.drawable.mesh_more else R.drawable.mesh_search,
                if (chatOpen && tab == "chat") "会话菜单" else "搜索会话",
                Color.WHITE,
            ) {
                if (tab == "chat" && chatOpen)
                    AlertDialog.Builder(this)
                        .setItems(arrayOf("搜索此会话", "会话设置")) { _, i ->
                            if (i == 0) {
                                revealSearch = !revealSearch
                                show("chat")
                            } else conversationActions()
                        }
                        .show()
                else {
                    chatOpen = false
                    show("chat")
                    threadList?.let {
                        body.findViewWithTag<EditText>("threadSearch")?.let { field ->
                            field.post {
                                field.requestFocus()
                                (getSystemService(INPUT_METHOD_SERVICE)
                                        as android.view.inputmethod.InputMethodManager)
                                    .showSoftInput(
                                        field,
                                        android.view.inputmethod.InputMethodManager.SHOW_IMPLICIT,
                                    )
                            }
                        }
                    }
                }
            }
        )
        root.addView(header)
        body =
            column().apply {
                setBackgroundColor(canvas)
                setPadding(
                    if (tab == "chat") 0 else dp(16),
                    if (tab == "chat") 0 else dp(12),
                    if (tab == "chat") 0 else dp(16),
                    0,
                )
            }
        if (tab == "chat") {
            root.addView(body, LinearLayout.LayoutParams(-1, 0, 1f))
            if (chatOpen) buildChat() else buildThreads()
        } else {
            root.addView(
                ScrollView(this).apply {
                    isFillViewport = true
                    addView(body)
                },
                LinearLayout.LayoutParams(-1, 0, 1f),
            )
            when (tab) {
                "nearby" -> buildNearby()
                "identity" -> buildIdentity()
                else -> buildSettings()
            }
        }
        val nav =
            BottomNavigationView(this).apply {
                setBackgroundColor(surface)
                itemIconTintList =
                    ColorStateList(
                        arrayOf(intArrayOf(android.R.attr.state_checked), intArrayOf()),
                        intArrayOf(accent, secondary),
                    )
                itemTextColor = itemIconTintList
                val items =
                    listOf(
                        Triple("chat", "聊天", android.R.drawable.ic_dialog_email),
                        Triple("nearby", "附近", android.R.drawable.ic_menu_search),
                        Triple("identity", "身份", android.R.drawable.ic_menu_myplaces),
                        Triple("settings", "自定义", android.R.drawable.ic_menu_preferences),
                    )
                items.forEachIndexed { index, item ->
                    menu.add(0, index + 1, index, item.second).setIcon(item.third)
                }
                selectedItemId = items.indexOfFirst { it.first == tab } + 1
                setOnItemSelectedListener { item ->
                    val key = items[item.itemId - 1].first
                    if (key != screen) {
                        search = ""
                        show(key)
                    }
                    true
                }
            }
        if (!(tab == "chat" && chatOpen)) root.addView(nav)
        val panel =
            column().apply {
                setBackgroundColor(surface)
                fitsSystemWindows = true
                setPadding(dp(16), dp(32), dp(16), dp(16))
            }
        panel.addView(
            avatar(data.optJSONObject("identity")?.optString("nickname") ?: "Campus", "self", 64)
        )
        panel.addView(
            label(
                    data.optJSONObject("identity")?.optString("nickname")
                        ?: prefs.getString("nickname", "同学")!!,
                    20f,
                )
                .apply { setTypeface(null, Typeface.BOLD) }
        )
        panel.addView(label("Campus Mesh · 校园离线通信", 13f, true))
        panel.addView(label(BuildConfig.VERSION_NAME, 12f, true))
        listOf("chat" to "聊天", "nearby" to "附近设备", "identity" to "我的身份", "settings" to "自定义外观")
            .forEach { (key, title) ->
                panel.addView(
                    button(title) {
                        drawer?.closeDrawer(GravityCompat.START)
                        chatOpen = false
                        show(key)
                    }
                )
            }
        panel.addView(
            button("扫码添加好友") {
                drawer?.closeDrawer(GravityCompat.START)
                scan()
            }
        )
        drawer =
            DrawerLayout(this).apply {
                setBackgroundColor(surface)
                drawerElevation = dp(16).toFloat()
                addView(root, DrawerLayout.LayoutParams(-1, -1))
                addView(
                    panel,
                    DrawerLayout.LayoutParams(dp(280), -1).apply { gravity = GravityCompat.START },
                )
            }
        drawer?.setDrawerLockMode(
            if (tab == "chat" && chatOpen) DrawerLayout.LOCK_MODE_LOCKED_CLOSED
            else DrawerLayout.LOCK_MODE_UNLOCKED
        )
        setContentView(drawer)
        updateSummary()
        if (tab == "chat") {
            if (chatOpen) renderMessages(true) else renderThreads(true)
        }
    }

    private fun icon(resource: Int, description: String, tint: Int = accent, action: () -> Unit) =
        ImageButton(this).apply {
            setImageResource(resource)
            imageTintList = ColorStateList.valueOf(tint)
            setBackgroundColor(Color.TRANSPARENT)
            contentDescription = description
            setPadding(dp(12), dp(12), dp(12), dp(12))
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
            setOnClickListener { action() }
        }

    private fun avatar(name: String, key: String, size: Int = 54) =
        TextView(this).apply {
            val palette =
                intArrayOf(
                    Color.rgb(95, 158, 204),
                    Color.rgb(123, 126, 199),
                    Color.rgb(82, 170, 149),
                    Color.rgb(206, 141, 95),
                    Color.rgb(187, 116, 151),
                )
            text =
                if (name.isBlank()) "?"
                else name.substring(0, name.offsetByCodePoints(0, 1)).uppercase(Locale.getDefault())
            gravity = Gravity.CENTER
            textSize = if (size < 40) 16f else 23f
            setTextColor(Color.WHITE)
            setTypeface(null, Typeface.BOLD)
            background =
                GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(palette[(key.hashCode() and Int.MAX_VALUE) % palette.size])
                }
            layoutParams = LinearLayout.LayoutParams(dp(size), dp(size))
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }

    private fun threadKey(message: JSONObject) =
        if (!message.isNull("group")) "group:" + message.optString("group")
        else "peer:" + message.optString("peer")

    private fun titleFor(key: String) =
        if (key.startsWith("group:")) groupTitle(key.removePrefix("group:"))
        else contacts().find { it.first == key.removePrefix("peer:") }?.second ?: "同学"

    private fun buildThreads() {
        val field =
            input("搜索聊天").apply {
                tag = "threadSearch"
                setText(threadQuery)
                setPadding(dp(20), 0, dp(20), 0)
                backgroundTintList = ColorStateList.valueOf(Color.TRANSPARENT)
            }
        watch(field) {
            threadQuery = it
            renderThreads(true)
        }
        body.addView(field)
        val filters = LinearLayout(this).apply { setPadding(dp(8), 0, dp(8), 0) }
        listOf("all" to "全部", "peer" to "私聊", "group" to "群聊").forEach { (id, name) ->
            filters.addView(
                button(if (id == threadFilter) "• $name" else name) {
                    threadFilter = id
                    show("chat")
                },
                LinearLayout.LayoutParams(0, dp(44), 1f),
            )
        }
        body.addView(filters)
        val frame = FrameLayout(this)
        threadList = column()
        frame.addView(
            ScrollView(this).apply {
                addView(threadList)
                isFillViewport = true
            },
            FrameLayout.LayoutParams(-1, -1),
        )
        frame.addView(
            FloatingActionButton(this).apply {
                setImageResource(R.drawable.mesh_compose)
                contentDescription = "新建会话"
                backgroundTintList = ColorStateList.valueOf(accent)
                imageTintList = ColorStateList.valueOf(if (dark) canvas else Color.WHITE)
                setOnClickListener { chooseConversation() }
            },
            FrameLayout.LayoutParams(dp(56), dp(56), Gravity.BOTTOM or Gravity.END).apply {
                setMargins(0, 0, dp(20), dp(20))
            },
        )
        body.addView(frame, LinearLayout.LayoutParams(-1, 0, 1f))
    }

    private fun renderThreads(force: Boolean = false) {
        val list = threadList ?: return
        val history = data.optJSONArray("history") ?: JSONArray()
        val records = (0 until history.length()).map { history.getJSONObject(it) }
        val latest = records.groupBy { threadKey(it) }.mapValues { it.value.last() }
        val names = contacts().toMap()
        fun title(key: String) =
            if (key.startsWith("group:")) groupTitle(key.removePrefix("group:"))
            else names[key.removePrefix("peer:")] ?: "同学"
        val unreadCounts =
            records
                .filter { it.optBoolean("incoming") && !readIds.contains(it.optString("id")) }
                .groupingBy { threadKey(it) }
                .eachCount()

        val keys =
            (contacts().map { "peer:" + it.first } +
                    groups().keys().asSequence().map { "group:$it" }.toList() +
                    latest.keys)
                .distinct()
        val pins = ui.getStringSet("pins", emptySet()) ?: emptySet()
        val visible =
            keys
                .filter { key ->
                    (threadFilter == "all" || key.startsWith("$threadFilter:")) &&
                        (threadQuery.isBlank() ||
                            title(key).contains(threadQuery, true) ||
                            latest[key]?.optString("text")?.contains(threadQuery, true) == true)
                }
                .sortedWith(
                    compareByDescending<String> { pins.contains(it.removePrefix("peer:")) }
                        .thenByDescending { latest[it]?.optLong("time") ?: 0 }
                        .thenBy { title(it) }
                )
        val stamp =
            visible.joinToString { key ->
                key + title(key) + latest[key].toString() + ui.getString("draft:$key", "")
            } + unreadCounts.toString() + threadQuery + threadFilter
        if (!force && stamp == threadStamp) return
        threadStamp = stamp
        list.removeAllViews()
        if (visible.isEmpty()) {
            list.addView(
                label(if (threadQuery.isNotBlank()) "未找到会话" else "开始你的第一段校园对话", 20f).apply {
                    gravity = Gravity.CENTER
                    setPadding(dp(20), dp(60), dp(20), dp(12))
                }
            )
            list.addView(
                label("扫描好友公钥二维码，或点击右下角新建会话。", 14f, true).apply {
                    gravity = Gravity.CENTER
                    setPadding(dp(20), 0, dp(20), 0)
                }
            )
            list.addView(button("扫码添加好友") { scan() })
            return
        }
        visible.forEach { key ->
            val last = latest[key]
            val unread = unreadCounts[key] ?: 0
            val draft = ui.getString("draft:$key", "") ?: ""
            val row =
                LinearLayout(this).apply {
                    gravity = Gravity.CENTER_VERTICAL
                    setPadding(dp(16), dp(11), dp(16), dp(11))
                    setBackgroundColor(surface)
                    isClickable = true
                    isFocusable = true
                    contentDescription = title(key) + if (unread > 0) "，$unread 条未读" else ""
                    setOnClickListener { selectConversation(key) }
                }
            row.addView(avatar(title(key), key))
            val center = column().apply { setPadding(dp(14), 0, dp(8), 0) }
            center.addView(
                label((if (key.startsWith("group:")) "▱ " else "") + title(key), 17f).apply {
                    setTypeface(null, Typeface.BOLD)
                    maxLines = 1
                    ellipsize = android.text.TextUtils.TruncateAt.END
                }
            )
            center.addView(
                label(
                        if (draft.isNotEmpty()) "草稿：$draft"
                        else last?.optString("text") ?: "点击开始聊天",
                        14f,
                        true,
                    )
                    .apply {
                        maxLines = 1
                        ellipsize = android.text.TextUtils.TruncateAt.END
                        if (draft.isNotEmpty()) setTextColor(accent)
                    }
            )
            row.addView(center, LinearLayout.LayoutParams(0, -2, 1f))
            val trailing = column().apply { gravity = Gravity.END }
            trailing.addView(
                label(
                    last?.let {
                        SimpleDateFormat("HH:mm", Locale.getDefault())
                            .format(Date(it.optLong("time") * 1000))
                    } ?: "",
                    12f,
                    true,
                )
            )
            if (unread > 0)
                trailing.addView(
                    label(unread.toString(), 12f).apply {
                        setTextColor(if (dark) canvas else Color.WHITE)
                        gravity = Gravity.CENTER
                        setPadding(dp(7), 0, dp(7), 0)
                        minWidth = dp(24)
                        background =
                            GradientDrawable().apply {
                                setColor(accent)
                                cornerRadius = dp(12).toFloat()
                            }
                    }
                )
            else if (pins.contains(key.removePrefix("peer:")))
                trailing.addView(label("置顶", 11f, true))
            row.addView(trailing)
            list.addView(row)
            list.addView(
                View(this).apply {
                    setBackgroundColor(
                        if (dark) Color.parseColor("#2C3944") else Color.parseColor("#EEF0F2")
                    )
                },
                LinearLayout.LayoutParams(-1, dp(1)).apply { leftMargin = dp(84) },
            )
        }
    }

    private fun contacts(): List<Pair<String, String>> {
        val all = data.optJSONObject("contacts") ?: JSONObject()
        val pins = ui.getStringSet("pins", emptySet()) ?: emptySet()
        return all.keys()
            .asSequence()
            .map { it to all.getJSONObject(it).optString("display_name", "同学") }
            .sortedWith(
                compareByDescending<Pair<String, String>> { pins.contains(it.first) }
                    .thenBy { it.second }
            )
            .toList()
    }

    private fun groups(): JSONObject = data.optJSONObject("groups") ?: JSONObject()

    private fun groupTitle(id: String) = groups().optJSONObject(id)?.optString("name") ?: "旧群聊记录"

    private fun deviceId() = data.optString("device_id")

    private fun displayName(id: String, fallback: String = "同学"): String {
        if (id == deviceId())
            return data.optJSONObject("identity")?.optString("nickname") ?: fallback
        data.optJSONObject("contacts")?.optJSONObject(id)?.optString("display_name")?.let {
            return it
        }
        val history = data.optJSONArray("history") ?: return fallback
        for (index in history.length() - 1 downTo 0) {
            val message = history.getJSONObject(index)
            if (message.optString("peer") == id && message.optBoolean("incoming"))
                return message.optString("sender_name", fallback)
        }
        return fallback
    }

    private fun groupName() = conversation.removePrefix("group:")

    private fun conversationTitle(): String =
        when {
            conversation.startsWith("group:") -> "群聊 · ${groupTitle(groupName())}"
            conversation.startsWith("peer:") ->
                contacts().find { it.first == conversation.removePrefix("peer:") }?.second ?: "选择会话"
            else -> "选择一位同学开始聊天"
        }

    private fun chooseConversation() {
        val contacts = contacts()
        val presets = groups()
        val groupKeys = presets.keys().asSequence().sorted().toList()
        val titles =
            contacts.map {
                (if (ui.getStringSet("pins", emptySet())!!.contains(it.first)) "★ " else "") +
                    it.second
            } + groupKeys.map { "群聊 · ${groupTitle(it)}" } + "＋ 创建群聊"
        AlertDialog.Builder(this)
            .setTitle("新建会话")
            .setItems(titles.toTypedArray()) { _, index ->
                if (index < contacts.size) selectConversation("peer:" + contacts[index].first)
                else if (index < contacts.size + groupKeys.size)
                    selectConversation("group:" + groupKeys[index - contacts.size])
                else editGroup()
            }
            .setNeutralButton("扫码添加好友") { _, _ -> scan() }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun selectConversation(key: String) {
        conversation = key
        chatOpen = true
        revealSearch = false
        search = ""
        ui.edit().putString("conversation", key).apply()
        show("chat")
    }

    private fun buildChat() {
        if (revealSearch) {
            val searchBox =
                input("搜索此会话").apply {
                    setText(search)
                    setPadding(dp(16), 0, dp(16), 0)
                }
            watch(searchBox) {
                search = it
                renderMessages(true)
            }
            body.addView(searchBox)
        }
        messages = column()
        messageScroll =
            ScrollView(this).apply {
                addView(messages)
                isFillViewport = true
                setBackgroundColor(Color.parseColor(if (dark) "#0E1922" else "#DCE5DE"))
                setPadding(dp(8), dp(8), dp(8), dp(8))
                clipToPadding = false
            }
        body.addView(messageScroll, LinearLayout.LayoutParams(-1, 0, 1f))
        val row =
            LinearLayout(this).apply {
                gravity = Gravity.BOTTOM
                setPadding(dp(4), dp(6), dp(8), dp(6))
                setBackgroundColor(surface)
            }
        row.addView(
            icon(R.drawable.mesh_phrase, "快捷短语") {
                AlertDialog.Builder(this)
                    .setTitle("快捷短语")
                    .setItems(quickReplies().toTypedArray()) { _, i ->
                        composer?.apply {
                            setText(quickReplies()[i])
                            setSelection(length())
                            requestFocus()
                        }
                    }
                    .setNegativeButton("取消", null)
                    .show()
            }
        )
        composer =
            input("消息").apply {
                setSingleLine(false)
                maxLines = 5
                background =
                    GradientDrawable().apply {
                        setColor(canvas)
                        cornerRadius = dp(24).toFloat()
                    }
                setPadding(dp(16), dp(10), dp(16), dp(10))
                isEnabled = conversation.isNotBlank()
                inputType =
                    android.text.InputType.TYPE_CLASS_TEXT or
                        android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE or
                        android.text.InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
                imeOptions = EditorInfo.IME_ACTION_SEND
                setOnEditorActionListener { _, action, _ ->
                    if (action == EditorInfo.IME_ACTION_SEND) {
                        send()
                        true
                    } else false
                }
            }
        applyingDraft = true
        composer!!.setText(ui.getString("draft:$conversation", ""))
        applyingDraft = false
        watch(composer!!) { value ->
            if (!applyingDraft && conversation.isNotEmpty())
                ui.edit().putString("draft:$conversation", value).apply()
            updateSend()
        }
        row.addView(composer, LinearLayout.LayoutParams(0, -2, 1f))
        sendButton =
            button("", true) { send() }
                .apply {
                    contentDescription = "发送消息"
                    cornerRadius = dp(28)
                    icon = getDrawable(R.drawable.mesh_send)
                    iconTint = ColorStateList.valueOf(if (dark) canvas else Color.WHITE)
                    iconPadding = 0
                    iconGravity = MaterialButton.ICON_GRAVITY_TEXT_START
                    minWidth = 0
                    setPadding(dp(16), 0, dp(16), 0)
                }
        row.addView(sendButton, LinearLayout.LayoutParams(dp(56), dp(56)))
        body.addView(row)
        updateSend()
    }

    private fun updateSend() {
        val enabled = conversation.isNotEmpty() && !composer?.text.isNullOrBlank()
        sendButton?.isEnabled = enabled
        sendButton?.alpha = if (enabled) 1f else 0.4f
    }

    private fun renderMessages(force: Boolean = false) {
        val container = messages ?: return
        val history = data.optJSONArray("history") ?: JSONArray()
        val matching =
            (0 until history.length())
                .map { history.getJSONObject(it) }
                .filter { message ->
                    val belongs =
                        if (conversation.startsWith("group:"))
                            message.optString("group") == groupName()
                        else
                            conversation.startsWith("peer:") &&
                                message.optString("peer") == conversation.removePrefix("peer:") &&
                                message.isNull("group")
                    belongs &&
                        (search.isBlank() || message.optString("text").contains(search, true))
                }
                .takeLast(200)
        val allIds =
            (0 until history.length()).map { history.getJSONObject(it).optString("id") }.toSet()
        val before = readIds.toSet()
        readIds.retainAll(allIds)
        (0 until history.length())
            .map { history.getJSONObject(it) }
            .filter { it.optBoolean("incoming") && threadKey(it) == conversation }
            .forEach { readIds.add(it.optString("id")) }
        if (before != readIds) ui.edit().putStringSet("readIds", readIds.toSet()).apply()
        val stamp = matching.joinToString { it.toString() } + conversation + search
        if (!force && stamp == lastRender) return
        val followBottom =
            container.height - (messageScroll!!.scrollY + messageScroll!!.height) < dp(90)
        lastRender = stamp
        container.removeAllViews()
        if (matching.isEmpty()) {
            container.addView(
                label(
                        if (search.isNotBlank()) "没有找到匹配的消息"
                        else if (conversation.isBlank()) "你的校园小网，从一位同学开始" else "还没有消息，打声招呼吧",
                        20f,
                    )
                    .apply {
                        gravity = Gravity.CENTER
                        setPadding(dp(20), dp(36), dp(20), dp(12))
                    }
            )
            container.addView(
                label(
                        if (search.isNotBlank()) "换个关键词试试。" else "当面互扫二维码。无需互联网，消息可等待下一次连接。",
                        14f,
                        true,
                    )
                    .apply { gravity = Gravity.CENTER }
            )
            if (conversation.isBlank()) container.addView(button("扫码添加同学", true) { scan() })
        }
        var previousDay = ""
        for (message in matching) {
            val day =
                SimpleDateFormat("MM月dd日", Locale.getDefault())
                    .format(Date(message.optLong("time") * 1000))
            if (day != previousDay) {
                container.addView(
                    label(day, 12f, true).apply {
                        gravity = Gravity.CENTER
                        setPadding(0, dp(10), 0, dp(6))
                    }
                )
                previousDay = day
            }
            val incoming = message.optBoolean("incoming")
            val row =
                LinearLayout(this).apply {
                    gravity = if (incoming) Gravity.START else Gravity.END
                    setPadding(0, dp(if (compact) 3 else 7), 0, 0)
                }
            val bubble =
                column().apply {
                    setPadding(
                        dp(14),
                        dp(if (compact) 7 else 12),
                        dp(14),
                        dp(if (compact) 7 else 12),
                    )
                    background =
                        GradientDrawable().apply {
                            setColor(
                                if (incoming)
                                    (if (dark) Color.parseColor("#182533") else Color.WHITE)
                                else if (ui.getString("accent", "blue") == "blue")
                                    Color.parseColor(if (dark) "#2B5278" else "#E1FFC7")
                                else ColorUtils.blendARGB(surface, accent, 0.18f)
                            )
                            val r = dp(16).toFloat()
                            val tail = dp(4).toFloat()
                            cornerRadii =
                                if (incoming) floatArrayOf(r, r, r, r, r, r, tail, tail)
                                else floatArrayOf(r, r, r, r, tail, tail, r, r)
                        }
                }
            if (incoming && conversation.startsWith("group:"))
                bubble.addView(
                    label(
                        displayName(
                            message.optString("peer"),
                            message.optString("sender_name", "同学"),
                        ),
                        12f,
                        true,
                    )
                )
            bubble.addView(
                label(message.optString("text")).apply {
                    maxWidth = dp(280)
                    setTextColor(ink)
                }
            )
            val state =
                when (message.optString("status")) {
                    "queued" -> "等待送达"
                    "delivered" -> "已送达"
                    "partially_delivered" -> {
                        val delivery = message.optJSONObject("delivery") ?: JSONObject()
                        val delivered =
                            delivery.keys().asSequence().count {
                                delivery.optString(it) == "delivered"
                            }
                        "已送达 $delivered/${delivery.length()}"
                    }
                    "received" -> "已收到"
                    "expired" -> "已过期"
                    else -> "待同步"
                }
            val time =
                if (ui.getBoolean("timestamps", true))
                    SimpleDateFormat("MM-dd HH:mm", Locale.getDefault())
                        .format(Date(message.optLong("time") * 1000)) + " · "
                else ""
            bubble.addView(
                label(
                        time +
                            when (message.optString("status")) {
                                "delivered" -> "✓ 已送达"
                                "queued" -> "◷ 等待送达"
                                else -> state
                            },
                        11f,
                        true,
                    )
                    .apply {
                        setTextColor(secondary)
                        gravity = Gravity.END
                    }
            )
            bubble.contentDescription =
                "${if(incoming) "收到" else "发出"}：${message.optString("text")}，$state"
            bubble.setOnLongClickListener {
                (getSystemService(CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(
                    ClipData.newPlainText("Campus Mesh 消息", message.optString("text"))
                )
                notice("消息已复制")
                true
            }
            row.addView(bubble)
            container.addView(row)
        }
        if (search.isBlank() && (force || followBottom))
            messageScroll?.post { messageScroll?.fullScroll(View.FOCUS_DOWN) }
    }

    private fun send() {
        val text = composer?.text?.toString() ?: return
        if (text.isBlank() || conversation.isBlank()) return
        if (text.toByteArray(Charsets.UTF_8).size > 8192) {
            notice("消息过长，请控制在 8192 字节以内")
            return
        }
        val group = conversation.startsWith("group:")
        val result =
            request(if (group) "group_send" else "send") {
                it.put(
                        if (group) "id" else "to",
                        if (group) groupName() else conversation.removePrefix("peer:"),
                    )
                    .put("text", text)
            }
        if (result != null) composer?.setText("")
        refreshState()
    }

    private fun editGroup(existing: String? = null) {
        val record = groups().optJSONObject(existing ?: "")
        val oldMembers = record?.optJSONObject("members") ?: JSONObject()
        val known =
            (contacts() +
                    oldMembers
                        .keys()
                        .asSequence()
                        .map { it to oldMembers.getJSONObject(it).optString("nickname", "同学") }
                        .toList())
                .distinctBy { it.first }
                .filter { it.first != deviceId() }
        val name = input("例如：自习小组").apply { setText(record?.optString("name") ?: "") }
        AlertDialog.Builder(this)
            .setTitle(if (existing == null) "创建群聊" else "修改群聊")
            .setMessage("群主可管理成员，最多 32 人。可以先建群，再通过邀请二维码添加成员。")
            .setView(name)
            .setPositiveButton("选择成员") { _, _ ->
                val title = name.text.toString().trim()
                if (title.isBlank() || title.toByteArray(Charsets.UTF_8).size > 96) {
                    notice("名称不能为空，且最多 96 字节")
                    return@setPositiveButton
                }
                val selected = BooleanArray(known.size) { oldMembers.has(known[it].first) }
                AlertDialog.Builder(this)
                    .setTitle("邀请好友或保留已有成员")
                    .setMultiChoiceItems(known.map { it.second }.toTypedArray(), selected) {
                        _,
                        i,
                        checked ->
                        selected[i] = checked
                    }
                    .setPositiveButton("保存") { _, _ ->
                        val chosen = known.filterIndexed { i, _ -> selected[i] }.map { it.first }
                        val result =
                            request(if (existing == null) "group_create" else "group_edit") {
                                it.put("name", title).put("members", JSONArray(chosen))
                                if (existing != null) it.put("id", existing)
                            }
                        if (result != null) {
                            refreshState()
                            selectConversation("group:" + (existing ?: result.getString("id")))
                        }
                    }
                    .setNegativeButton("取消", null)
                    .show()
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun conversationActions() {
        if (conversation.isBlank()) {
            chooseConversation()
            return
        }
        if (conversation.startsWith("group:")) {
            val id = groupName()
            val group = groups().optJSONObject(id)
            val members = group?.optJSONObject("members") ?: JSONObject()
            val owner =
                group?.optJSONObject("owner")?.optString("sign") ==
                    data.optJSONObject("identity")?.optString("sign")
            val active = group?.optBoolean("joined") == true && !group.optBoolean("dissolved")
            val options = mutableListOf("查看成员与群 ID")
            if (active && !owner) options.add("同步群信息")
            if (active && owner) options.addAll(listOf("修改群名和成员", "生成入群二维码"))
            if (active) options.add(if (owner) "解散群聊" else "退出群聊")
            AlertDialog.Builder(this)
                .setTitle(conversationTitle())
                .setItems(options.toTypedArray()) { _, index ->
                    when (options[index]) {
                        "查看成员与群 ID" ->
                            AlertDialog.Builder(this)
                                .setTitle(groupTitle(id))
                                .setMessage(
                                    "群 ID：$id\n\n" +
                                        members.keys().asSequence().joinToString("\n\n") {
                                            displayName(
                                                it,
                                                members.getJSONObject(it).optString("nickname"),
                                            ) + "\n" + it
                                        } +
                                        "\n\n成员变更在 Mesh 同步后生效，聊天记录保存在各自设备。"
                                )
                                .setPositiveButton("完成", null)
                                .show()
                        "修改群名和成员" -> editGroup(id)
                        "同步群信息" ->
                            if (request("group_sync") { it.put("id", id) } != null)
                                notice("群信息同步请求已加入 Mesh 待传队列")
                        "生成入群二维码" ->
                            request("group_invite") { it.put("id", id) }
                                ?.let {
                                    showQR(
                                        "邀请入群",
                                        it.toString(),
                                        "10 分钟内有效，仅可使用一次。重新生成会撤销上一个邀请；扫码后需在有效期内与群主通过 Mesh 同步。",
                                    )
                                }
                        else ->
                            AlertDialog.Builder(this)
                                .setTitle(options[index] + "？")
                                .setMessage("保留本机聊天记录，变更将通过 Mesh 同步。")
                                .setPositiveButton("确认") { _, _ ->
                                    if (request("group_leave") { it.put("id", id) } != null) {
                                        refreshState()
                                        show("chat")
                                    }
                                }
                                .setNegativeButton("取消", null)
                                .show()
                    }
                }
                .setNegativeButton("取消", null)
                .show()
        } else {
            val id = conversation.removePrefix("peer:")
            val pins = (ui.getStringSet("pins", emptySet()) ?: emptySet()).toMutableSet()
            AlertDialog.Builder(this)
                .setTitle(conversationTitle())
                .setItems(
                    arrayOf(if (pins.contains(id)) "取消常用好友" else "设为常用好友", "设置好友备注", "查看已记住的设备")
                ) { _, index ->
                    when (index) {
                        0 -> {
                            if (!pins.add(id)) pins.remove(id)
                            ui.edit().putStringSet("pins", pins).apply()
                            notice("常用好友排序已更新")
                        }
                        1 -> editRemark(id)
                        else ->
                            AlertDialog.Builder(this)
                                .setTitle("已记住的好友设备")
                                .setMessage(
                                    "昵称：${data.optJSONObject("contacts")?.optJSONObject(id)?.optString("nickname")}\n\n设备 ID：$id\n\n签名公钥：${data.optJSONObject("contacts")?.optJSONObject(id)?.optString("sign")}"
                                )
                                .setPositiveButton("完成", null)
                                .show()
                    }
                }
                .show()
        }
    }

    private fun editRemark(id: String) {
        val field =
            input("仅自己可见，留空后显示昵称").apply {
                setText(
                    data.optJSONObject("contacts")?.optJSONObject(id)?.optString("remark") ?: ""
                )
            }
        AlertDialog.Builder(this)
            .setTitle("好友备注")
            .setView(field)
            .setPositiveButton("保存") { _, _ ->
                if (
                    request("friend_remark") {
                        it.put("id", id).put("remark", field.text.toString())
                    } != null
                ) {
                    refreshState()
                    show(screen)
                }
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun showQR(title: String, text: String, description: String) {
        runCatching {
                val size = 720
                val bits = MeshQr.encode(text, size)
                val pixels =
                    IntArray(size * size) { i ->
                        if (bits[i % size, i / size]) Color.BLACK else Color.WHITE
                    }
                val content =
                    column().apply {
                        addView(
                            ImageView(this@MainActivity).apply {
                                setImageBitmap(
                                    Bitmap.createBitmap(pixels, size, size, Bitmap.Config.ARGB_8888)
                                )
                                setBackgroundColor(Color.WHITE)
                                adjustViewBounds = true
                                contentDescription = title
                            },
                            LinearLayout.LayoutParams(-1, dp(300)),
                        )
                        addView(label(description, 13f, true))
                    }
                AlertDialog.Builder(this)
                    .setTitle(title)
                    .setView(content)
                    .setPositiveButton("完成", null)
                    .setNeutralButton("分享文本") { _, _ ->
                        startActivity(
                            Intent.createChooser(
                                Intent(Intent.ACTION_SEND).apply {
                                    type = "text/plain"
                                    putExtra(Intent.EXTRA_TEXT, text)
                                },
                                title,
                            )
                        )
                    }
                    .show()
            }
            .onFailure { notice("二维码生成失败，请重新生成邀请") }
    }

    private fun meshSettings() {
        val settings = data.optJSONObject("settings") ?: return
        val fields =
            listOf(
                Triple("hop_limit", "最大跳数", "1–8"),
                Triple("message_ttl_secs", "消息有效期（秒）", "60–86400"),
                Triple("sync_interval_secs", "同步间隔（秒）", "1–60"),
                Triple("batch_size", "TCP 每批封装数", "1–24"),
                Triple("max_peers", "TCP 邻居上限", "1–300"),
                Triple("cache_capacity", "待传封装上限", "64–4096"),
            )
        val editors =
            fields.associate { (key, title, range) ->
                key to
                    input("$title · $range").apply {
                        inputType = android.text.InputType.TYPE_CLASS_NUMBER
                        setText(settings.optInt(key).toString())
                    }
            }
        val form =
            column().apply {
                addView(
                    label(
                        "跳数和有效期用于新消息。同步间隔影响 TCP 和蓝牙；批量和邻居上限用于 TCP。蓝牙仍每轮发送一个封装，每侧最多 4 条连接。",
                        13f,
                        true,
                    )
                )
                fields.forEach { (key, title, range) ->
                    addView(label("$title（$range）", 13f, true))
                    addView(editors.getValue(key))
                }
            }
        val dialog =
            AlertDialog.Builder(this)
                .setTitle("Mesh 详细参数")
                .setView(ScrollView(this).apply { addView(form) })
                .setPositiveButton("保存", null)
                .setNeutralButton("默认值", null)
                .setNegativeButton("取消", null)
                .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val values = JSONObject()
                for ((key, _, _) in fields) {
                    val value = editors.getValue(key).text.toString().toLongOrNull()
                    if (value == null) {
                        notice("请输入有效整数")
                        return@setOnClickListener
                    }
                    values.put(key, value)
                }
                if (request("settings") { it.put("settings", values) } != null) {
                    refreshState()
                    dialog.dismiss()
                    notice("Mesh 参数已保存")
                }
            }
            dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setOnClickListener {
                listOf(8, 86400, 5, 24, 300, 4096).forEachIndexed { i, value ->
                    editors.getValue(fields[i].first).setText(value.toString())
                }
            }
        }
        dialog.show()
    }

    private fun radio(action: String) {
        startForegroundService(Intent(this, MeshService::class.java).setAction(action))
    }

    private fun scan() {
        IntentIntegrator(this)
            .setDesiredBarcodeFormats(IntentIntegrator.QR_CODE)
            .setPrompt("扫描好友或群邀请二维码")
            .setBeepEnabled(false)
            .initiateScan()
    }

    private fun permissions(code: Int, action: String) {
        val requested =
            when {
                code == 22 && Build.VERSION.SDK_INT >= 31 ->
                    arrayOf(
                        Manifest.permission.BLUETOOTH_SCAN,
                        Manifest.permission.BLUETOOTH_CONNECT,
                        Manifest.permission.BLUETOOTH_ADVERTISE,
                    )
                code == 23 && Build.VERSION.SDK_INT >= 33 ->
                    arrayOf(Manifest.permission.NEARBY_WIFI_DEVICES)
                else ->
                    arrayOf(
                        Manifest.permission.ACCESS_FINE_LOCATION,
                        Manifest.permission.ACCESS_COARSE_LOCATION,
                    )
            }
        if (
            requested.all {
                checkSelfPermission(it) == android.content.pm.PackageManager.PERMISSION_GRANTED
            }
        )
            radio(action)
        else requestPermissions(requested, code)
    }

    private fun buildNearby() {
        card("先认识，再连接", "两端互扫二维码，消息只向已信任的公钥身份开放。") {
            addView(button("扫描好友或群邀请二维码", true) { scan() })
            addView(
                button("粘贴二维码文本") {
                    val text = input("好友或群邀请 JSON")
                    AlertDialog.Builder(this@MainActivity)
                        .setTitle("添加好友")
                        .setView(text)
                        .setPositiveButton("添加") { _, _ -> addContact(text.text.toString()) }
                        .setNegativeButton("取消", null)
                        .show()
                }
            )
        }
        card("蓝牙直连", "没有 Wi-Fi 也能尝试连接；两端需启用蓝牙。") {
            addView(button("启用蓝牙", true) { permissions(22, "BLE_ON") })
            addView(button("停止蓝牙") { radio("BLE_OFF") })
        }
        card("附近的网络") {
            peerSummary = label("正在读取…", 13f, true)
            addView(peerSummary)
            addView(
                button("连接局域网地址") {
                    val address =
                        input("例如 192.168.1.25:37078").apply {
                            setText(ui.getString("lastAddress", ""))
                            inputType =
                                android.text.InputType.TYPE_CLASS_TEXT or
                                    android.text.InputType.TYPE_TEXT_VARIATION_URI
                        }
                    AlertDialog.Builder(this@MainActivity)
                        .setTitle("连接同一 Wi-Fi / 热点上的设备")
                        .setView(address)
                        .setPositiveButton("连接") { _, _ ->
                            val value = address.text.toString().trim()
                            if (request("peer") { it.put("address", value) } != null) {
                                ui.edit().putString("lastAddress", value).apply()
                                refreshState()
                            }
                        }
                        .setNegativeButton("取消", null)
                        .show()
                }
            )
            addView(button("Wi-Fi Direct · 发现设备") { permissions(23, "P2P_DISCOVER") })
            addView(
                button("Wi-Fi Direct · 选择设备") {
                    val devices = WifiDirectMesh.peers.toList()
                    if (devices.isEmpty()) notice("暂未发现设备，请先发现并等待，确认双方 Wi-Fi 与系统权限已开启。")
                    else
                        AlertDialog.Builder(this@MainActivity)
                            .setTitle("选择设备，随后接受系统配对")
                            .setItems(devices.map { it.deviceName }.toTypedArray()) { _, i ->
                                startService(
                                    Intent(this@MainActivity, MeshService::class.java)
                                        .setAction("P2P_CONNECT")
                                        .putExtra("address", devices[i].deviceAddress)
                                )
                            }
                            .show()
                }
            )
            addView(button("断开 Wi-Fi Direct") { radio("P2P_OFF") })
        }
        card("Mesh 多跳", "新消息可经多个设备传递。设备 ID 与好友记录保存在本机，修改昵称不会改变身份。") {
            addView(button("详细传输参数") { meshSettings() })
            val pending = data.optJSONObject("pending_groups")?.length() ?: 0
            if (pending > 0) addView(label("$pending 个入群申请等待与群主同步，10 分钟内有效。", 13f, true))
        }
        card("已记住的好友", "点击设置本机备注。重新打开应用后无需重复添加。") {
            contacts().forEach { (id, name) ->
                addView(button("$name · ${id.take(12)}…") { editRemark(id) })
            }
        }
        card("连接小提示", "校园 Wi-Fi 若隔离设备，请使用自己的热点或蓝牙。蓝牙和 Wi-Fi Direct 仍需真机兼容性验证。") {}
    }

    private fun addContact(raw: String) {
        val result = request("scan") { it.put("text", raw) } ?: return
        refreshState()
        if (result.optString("kind") == "friend")
            selectConversation("peer:" + result.getString("id"))
        else notice("入群申请已提交，等待与群主完成 Mesh 同步。邀请 10 分钟内有效。")
    }

    private fun buildIdentity() {
        val identity = data.optJSONObject("identity")
        card(identity?.optString("nickname") ?: "正在读取本机身份", "二维码只包含公钥；请与好友当面核对。") {
            if (identity != null) {
                val json = data.optJSONObject("contact_card")?.toString() ?: identity.toString()
                val size = 480
                val bits = MeshQr.encode(json, size)
                val pixels =
                    IntArray(size * size) { i ->
                        if (bits[i % size, i / size]) Color.BLACK else Color.WHITE
                    }
                addView(
                    ImageView(this@MainActivity).apply {
                        setImageBitmap(
                            Bitmap.createBitmap(pixels, size, size, Bitmap.Config.ARGB_8888)
                        )
                        contentDescription = "我的好友公钥二维码"
                        setBackgroundColor(Color.WHITE)
                        setPadding(dp(12), dp(12), dp(12), dp(12))
                        adjustViewBounds = true
                    },
                    LinearLayout.LayoutParams(-1, dp(260)),
                )
                addView(
                    label(
                            "本机设备 ID：" + deviceId() + "\n\n签名公钥：" + identity.optString("sign"),
                            12f,
                            true,
                        )
                        .apply { setTextIsSelectable(true) }
                )
                addView(
                    button("分享公钥") {
                        startActivity(
                            Intent.createChooser(
                                Intent(Intent.ACTION_SEND).apply {
                                    type = "text/plain"
                                    putExtra(Intent.EXTRA_TEXT, json)
                                },
                                "分享公钥",
                            )
                        )
                    }
                )
            }
            addView(
                button("修改昵称") {
                    val name = input("新昵称").apply { setText(identity?.optString("nickname") ?: "") }
                    AlertDialog.Builder(this@MainActivity)
                        .setTitle("本机昵称")
                        .setView(name)
                        .setPositiveButton("保存") { _, _ ->
                            if (
                                request("profile") { it.put("nickname", name.text.toString()) } !=
                                    null
                            ) {
                                prefs.edit().putString("nickname", name.text.toString()).apply()
                                refreshState()
                                show("identity")
                            }
                        }
                        .setNegativeButton("取消", null)
                        .show()
                }
            )
        }
        card("中继与电量", "中继只保存与传递密文；关闭后仍能发送和接收本人的消息。") {
            addView(
                SwitchMaterial(this@MainActivity).apply {
                    text = "参与消息中继"
                    setTextColor(ink)
                    minHeight = dp(48)
                    isEnabled = data.has("relay")
                    isChecked = data.optBoolean("relay", true)
                    setOnCheckedChangeListener { _, checked ->
                        if (request("relay") { it.put("enabled", checked) } == null) {
                            refreshState()
                            show("identity")
                        } else {
                            data.put("relay", checked)
                        }
                    }
                }
            )
            addView(
                button("通知设置") {
                    startActivity(
                        Intent(android.provider.Settings.ACTION_APP_NOTIFICATION_SETTINGS)
                            .putExtra(android.provider.Settings.EXTRA_APP_PACKAGE, packageName)
                    )
                }
            )
            addView(
                button("停止通信并退出") {
                    stopService(Intent(this@MainActivity, MeshService::class.java))
                    finish()
                }
            )
        }
    }

    private fun quickReplies(): List<String> =
        (ui.getString("phrases", "我到了\n在哪个教室？\n下课见") ?: "")
            .lines()
            .map { it.trim() }
            .filter { it.isNotEmpty() }
            .take(8)

    private fun selection(
        title: String,
        labels: Array<String>,
        chosen: Int,
        select: (Int) -> Unit,
    ) {
        AlertDialog.Builder(this)
            .setTitle(title)
            .setSingleChoiceItems(labels, chosen) { dialog, index ->
                dialog.dismiss()
                select(index)
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun buildSettings() {
        card("让界面更像你", "偏好只保存在本机，不改变公钥和通信方式。") {
            val themes = listOf("system", "light", "dark")
            val themeNames = arrayOf("跟随系统", "浅色", "深色")
            addView(
                button(
                    "外观 · " +
                        themeNames[themes.indexOf(ui.getString("theme", "system")).coerceAtLeast(0)]
                ) {
                    selection("外观", themeNames, themes.indexOf(ui.getString("theme", "system"))) {
                        index ->
                        ui.edit().putString("theme", themes[index]).apply()
                        AppCompatDelegate.setDefaultNightMode(
                            when (index) {
                                1 -> AppCompatDelegate.MODE_NIGHT_NO
                                2 -> AppCompatDelegate.MODE_NIGHT_YES
                                else -> AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM
                            }
                        )
                        show("settings")
                    }
                }
            )
            val colors = listOf("teal", "blue", "violet", "amber")
            addView(
                button("主题色 · 自选校园色") {
                    selection(
                        "主题色",
                        arrayOf("松绿", "晴蓝", "鸢紫", "暖杏"),
                        colors.indexOf(ui.getString("accent", "blue")),
                    ) { index ->
                        ui.edit().putString("accent", colors[index]).apply()
                        recreate()
                    }
                }
            )
            val sizes = listOf(14, 16, 18, 20)
            addView(
                button("聊天字号 · ${font.toInt()}") {
                    selection(
                        "聊天字号",
                        arrayOf("小 · 14", "标准 · 16", "大 · 18", "特大 · 20"),
                        sizes.indexOf(font.toInt()),
                    ) { index ->
                        ui.edit().putInt("font", sizes[index]).apply()
                        show("settings")
                    }
                }
            )
            addView(
                SwitchMaterial(this@MainActivity).apply {
                    text = "紧凑气泡"
                    setTextColor(ink)
                    minHeight = dp(48)
                    isChecked = compact
                    setOnCheckedChangeListener { _, checked ->
                        ui.edit().putBoolean("compact", checked).apply()
                    }
                }
            )
            addView(
                SwitchMaterial(this@MainActivity).apply {
                    text = "显示消息时间"
                    setTextColor(ink)
                    minHeight = dp(48)
                    isChecked = ui.getBoolean("timestamps", true)
                    setOnCheckedChangeListener { _, checked ->
                        ui.edit().putBoolean("timestamps", checked).apply()
                    }
                }
            )
            addView(label("预览：下课后，图书馆见。", font).apply { setTextColor(accent) })
        }
        card("少点几次，更快表达") {
            addView(
                button("编辑快捷短语") {
                    val editor =
                        input("每行一条，最多 8 条").apply {
                            setSingleLine(false)
                            minLines = 4
                            setText(quickReplies().joinToString("\n"))
                        }
                    AlertDialog.Builder(this@MainActivity)
                        .setTitle("常用的那些话")
                        .setView(editor)
                        .setPositiveButton("保存") { _, _ ->
                            ui.edit()
                                .putString(
                                    "phrases",
                                    editor.text
                                        .toString()
                                        .lines()
                                        .map { it.trim().take(80) }
                                        .filter { it.isNotEmpty() }
                                        .take(8)
                                        .joinToString("\n"),
                                )
                                .apply()
                        }
                        .setNegativeButton("取消", null)
                        .show()
                }
            )
            addView(button("创建群聊") { editGroup() })
            addView(label("草稿按会话自动保存。快捷短语只填入输入框，仍由你点击发送。长按气泡可复制；会话菜单可设置常用好友。", 13f, true))
        }
        card("恢复默认外观", "仅恢复主题、字号、密度与时间显示，保留身份、聊天记录、草稿、短语和群聊。") {
            addView(
                button("恢复默认外观") {
                    ui.edit()
                        .remove("theme")
                        .remove("accent")
                        .remove("font")
                        .remove("compact")
                        .remove("timestamps")
                        .apply()
                    AppCompatDelegate.setDefaultNightMode(
                        AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM
                    )
                    show("settings")
                }
            )
        }
    }

    private fun updateSummary() {
        if (!::status.isInitialized) return
        if (data.length() == 0) {
            status.text = "通信核心正在启动，稍候即可收发"
            return
        }
        val peers = data.optJSONArray("peers") ?: JSONArray()
        val connected =
            (0 until peers.length()).count { peers.getJSONObject(it).optBoolean("connected") }
        val summary =
            if (screen == "chat" && chatOpen) "离线消息 · 自动同步"
            else "本机已就绪 · $connected 条局域网连接 · ${contacts().size} 位好友"
        if (status.text.toString() != summary) status.text = summary
        peerSummary?.text =
            (0 until peers.length())
                .joinToString("\n") {
                    peers.getJSONObject(it).let { p ->
                        "${p.optString("address")} · ${if(p.optBoolean("connected")) "已连接" else "等待重连"}"
                    }
                }
                .ifBlank { "暂无局域网邻居" } + "\n${BleMesh.status}\n${WifiDirectMesh.status}"
    }

    private fun refreshState() {
        val next =
            runCatching { Native.request(JSONObject().put("op", "state")).getJSONObject("data") }
                .getOrNull() ?: return
        val wasEmpty = data.length() == 0
        data = next
        if (wasEmpty) migrateLegacyGroups()
        val signature = next.optJSONObject("contacts")?.toString() ?: "{}"
        lastContacts = signature
        updateSummary()
        if (screen == "chat") {
            if (chatOpen) renderMessages() else renderThreads()
        } else if (screen == "identity" && wasEmpty) show("identity")
    }

    private fun migrateLegacyGroups() {
        val old =
            runCatching { JSONObject(ui.getString("groups", "{}")!!) }.getOrDefault(JSONObject())
        for (name in old.keys().asSequence().toList()) {
            val result =
                request("group_migrate") {
                    it.put("name", name).put("members", old.getJSONArray(name))
                } ?: continue
            val oldKey = "group:$name"
            val newKey = "group:" + result.getString("id")
            val editor = ui.edit()
            ui.getString("draft:$oldKey", null)?.let { editor.putString("draft:$newKey", it) }
            val retry =
                runCatching { JSONObject(ui.getString("retry:$oldKey", "{}")!!) }
                    .getOrDefault(JSONObject())
            if (!ui.contains("draft:$newKey") && !ui.contains("draft:$oldKey") && retry.has("text"))
                editor.putString("draft:$newKey", retry.getString("text"))
            editor.remove("draft:$oldKey").remove("retry:$oldKey")
            if (conversation == oldKey) {
                conversation = newKey
                editor.putString("conversation", newKey)
            }
            old.remove(name)
            editor.putString("groups", old.toString()).apply()
        }
        if (old.length() == 0) ui.edit().remove("groups").apply()
        data =
            runCatching { Native.request(JSONObject().put("op", "state")).getJSONObject("data") }
                .getOrDefault(data)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (
            grantResults.isNotEmpty() &&
                grantResults.all { it == android.content.pm.PackageManager.PERMISSION_GRANTED }
        ) {
            if (requestCode == 22) radio("BLE_ON")
            if (requestCode == 23) radio("P2P_DISCOVER")
        } else if (requestCode == 22 || requestCode == 23) notice("权限未完整授予，可在系统设置中开启，或使用已授权的局域网连接。")
    }

    @Deprecated("Legacy scanner callback")
    override fun onActivityResult(requestCode: Int, resultCode: Int, intent: Intent?) {
        val result = IntentIntegrator.parseActivityResult(requestCode, resultCode, intent)
        if (result != null) {
            if (result.contents != null) addContact(result.contents)
        } else super.onActivityResult(requestCode, resultCode, intent)
    }

    override fun onSaveInstanceState(outState: Bundle) {
        outState.putString("screen", screen)
        outState.putBoolean("chatOpen", chatOpen)
        super.onSaveInstanceState(outState)
    }

    override fun onResume() {
        super.onResume()
        active = true
        timer.removeCallbacks(refresh)
        timer.post(refresh)
    }

    override fun onPause() {
        active = false
        timer.removeCallbacks(refresh)
        super.onPause()
    }
}
