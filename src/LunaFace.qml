import QtQuick
import QtQuick.Shapes

// Luna's face. Everything here follows backend.state, which mirrors voiced:
// idle, wake, listening, thinking, speaking. Sizes derive from `s` so the face
// fits any window, portrait or landscape.
Item {
    id: root

    readonly property string mood: backend.state
    readonly property color accent: theme.accent
    readonly property real s: Math.min(width, height * 0.62)
    readonly property bool offline: !backend.connected && !backend.demo

    // After a while of quiet Luna dozes off: slit eyes, floating z's, dimmer.
    property bool sleepy: true
    Timer {
        id: dozeTimer
        interval: 20000
        running: root.mood === "idle"
        onTriggered: root.sleepy = true
    }
    onMoodChanged: {
        if (mood !== "idle") sleepy = false
        if (mood === "wake") wakeBounce.restart()
        dozeTimer.restart()
    }

    opacity: (sleepy || offline) && mood === "idle" ? 0.55 : 1
    Behavior on opacity { NumberAnimation { duration: 900; easing.type: Easing.InOutQuad } }

    // ---- Eyes: where they look, how open they are, blinking ----------------

    property real lookX: 0
    property real lookY: 0
    Behavior on lookX { NumberAnimation { duration: 450; easing.type: Easing.InOutQuad } }
    Behavior on lookY { NumberAnimation { duration: 450; easing.type: Easing.InOutQuad } }

    readonly property real openness: {
        if (offline || (mood === "idle" && sleepy)) return 0.1
        switch (mood) {
        case "wake": return 1.45
        case "listening": return 1.25
        case "thinking": return 0.8
        case "speaking": return 1.05
        default: return 0.95
        }
    }

    property bool blinking: false
    Timer {
        interval: 2500 + Math.random() * 3500
        repeat: true
        running: root.openness > 0.3
        onTriggered: {
            interval = 2500 + Math.random() * 3500
            root.blinking = true
            unblink.restart()
        }
    }
    Timer { id: unblink; interval: 130; onTriggered: root.blinking = false }

    // Thinking: glance up and around. Idle: an occasional lazy drift.
    SequentialAnimation {
        running: root.mood === "thinking"
        loops: Animation.Infinite
        onStopped: { root.lookX = 0; root.lookY = 0 }
        ScriptAction { script: { root.lookX = -0.5; root.lookY = -0.55 } }
        PauseAnimation { duration: 900 }
        ScriptAction { script: { root.lookX = 0.5; root.lookY = -0.45 } }
        PauseAnimation { duration: 900 }
    }
    Timer {
        interval: 3200
        repeat: true
        running: root.mood === "idle" && !root.sleepy
        onTriggered: { root.lookX = (Math.random() - 0.5) * 0.8; root.lookY = (Math.random() - 0.5) * 0.4 }
    }

    // Shared cycle for the sleepy z's.
    property real zPhase: 0
    NumberAnimation on zPhase {
        running: (root.sleepy || root.offline) && root.mood === "idle"
        loops: Animation.Infinite
        from: 0
        to: 1
        duration: 4200
    }

    // ---- The face ----------------------------------------------------------

    Item {
        id: face
        width: root.s
        height: root.s
        x: (root.width - width) / 2
        y: root.height * 0.40 - height / 2 + bounce

        property real bounce: 0
        SequentialAnimation {
            id: wakeBounce
            NumberAnimation { target: face; property: "bounce"; to: -root.s * 0.05; duration: 140; easing.type: Easing.OutQuad }
            NumberAnimation { target: face; property: "bounce"; to: 0; duration: 520; easing.type: Easing.OutBounce }
        }

        // Gentle breathing while idle.
        transform: Scale {
            origin.x: face.width / 2
            origin.y: face.height / 2
            xScale: face.breath
            yScale: face.breath
        }
        property real breath: 1
        SequentialAnimation on breath {
            loops: Animation.Infinite
            running: root.mood === "idle"
            NumberAnimation { to: 1.025; duration: 2200; easing.type: Easing.InOutSine }
            NumberAnimation { to: 1.0; duration: 2200; easing.type: Easing.InOutSine }
        }

        // Listening ring: a soft pulse around the face.
        Rectangle {
            id: ring
            anchors.centerIn: parent
            width: root.s * 1.02
            height: width
            radius: width / 2
            color: "transparent"
            border.color: root.accent
            border.width: Math.max(2, root.s * 0.008)
            opacity: 0
            scale: 0.92
            SequentialAnimation {
                running: root.mood === "listening"
                loops: Animation.Infinite
                onStopped: { ring.opacity = 0; ring.scale = 0.92 }
                ParallelAnimation {
                    NumberAnimation { target: ring; property: "opacity"; from: 0.55; to: 0; duration: 1300; easing.type: Easing.OutQuad }
                    NumberAnimation { target: ring; property: "scale"; from: 0.86; to: 1.08; duration: 1300; easing.type: Easing.OutQuad }
                }
            }
        }

        Repeater {
            model: [-1, 1]
            delegate: Item {
                id: eye
                required property int modelData
                readonly property real w: root.s * 0.19
                x: face.width / 2 + modelData * root.s * 0.19 - w / 2 + root.lookX * root.s * 0.05
                y: face.height * 0.40 + root.lookY * root.s * 0.05
                width: w

                // Glow: a few soft, wider copies of the eye behind it. Plain shapes
                // rather than a shader effect, so it renders anywhere and costs nothing.
                Repeater {
                    model: 3
                    delegate: Rectangle {
                        required property int index
                        readonly property real grow: root.s * 0.022 * (index + 1)
                        anchors.centerIn: eyeShape
                        width: eyeShape.width + grow * 2
                        height: eyeShape.height + grow * 2
                        radius: Math.min(width, height) / 2
                        color: root.accent
                        opacity: [0.16, 0.08, 0.04][index] * (root.openness > 0.3 ? 1 : 0.5)
                    }
                }

                Rectangle {
                    id: eyeShape
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.verticalCenter: parent.top
                    width: eye.w
                    height: Math.max(root.s * 0.018, eye.w * 1.15 * root.openness * (root.blinking ? 0.08 : 1))
                    radius: Math.min(width, height) / 2
                    color: root.accent
                    Behavior on height { SpringAnimation { spring: 5; damping: 0.32; epsilon: 0.5 } }

                    // A little sparkle, only when the eye is open enough to hold it.
                    Rectangle {
                        width: eye.w * 0.24
                        height: width
                        radius: width / 2
                        color: "white"
                        opacity: eyeShape.height > eye.w * 0.75 ? 0.85 : 0
                        x: parent.width * 0.58 - width / 2
                        y: parent.height * 0.22
                        Behavior on opacity { NumberAnimation { duration: 150 } }
                    }
                }

                // Cheek, tucked under the outer corner of the eye.
                Rectangle {
                    width: eye.w * 0.62
                    height: width * 0.48
                    radius: height / 2
                    color: "#ff7aa8"
                    x: eye.w / 2 - width / 2 + eye.modelData * eye.w * 0.62
                    y: root.s * 0.155
                    opacity: root.offline || root.sleepy && root.mood === "idle" ? 0.08
                           : root.mood === "wake" || root.mood === "speaking" ? 0.26 : 0.14
                    Behavior on opacity { NumberAnimation { duration: 400 } }
                }
            }
        }

        // ---- Mouth -----------------------------------------------------------

        Item {
            id: mouth
            x: face.width / 2
            y: face.height * 0.64

            property real talk: 0
            Timer {
                interval: 95
                repeat: true
                running: root.mood === "speaking"
                onTriggered: mouth.talk = 0.2 + Math.random() * 0.8
                onRunningChanged: if (!running) mouth.talk = 0
            }

            // Open mouth for talking, listening ("o") and waking (surprised).
            Rectangle {
                readonly property bool shown: root.mood === "speaking" || root.mood === "listening" || root.mood === "wake"
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.verticalCenter: parent.top
                width: root.mood === "speaking" ? root.s * 0.13 : root.s * 0.065
                height: root.mood === "speaking" ? root.s * (0.025 + 0.075 * mouth.talk)
                      : root.mood === "wake" ? root.s * 0.08 : root.s * 0.055
                radius: Math.min(width, height) / 2
                color: Qt.darker(root.accent, 2.2)
                border.color: root.accent
                border.width: Math.max(2, root.s * 0.01)
                opacity: shown ? 1 : 0
                Behavior on width { NumberAnimation { duration: 140 } }
                Behavior on height { NumberAnimation { duration: 80 } }
                Behavior on opacity { NumberAnimation { duration: 160 } }
            }

            // Thinking: a small sideways line, mulling it over.
            Rectangle {
                anchors.verticalCenter: parent.top
                x: -width / 2 + root.s * 0.03
                width: root.s * 0.07
                height: Math.max(3, root.s * 0.016)
                radius: height / 2
                color: root.accent
                opacity: root.mood === "thinking" ? 0.9 : 0
                Behavior on opacity { NumberAnimation { duration: 160 } }
            }

            // Resting smile (idle and asleep).
            Shape {
                opacity: root.mood === "idle" ? 0.9 : 0
                Behavior on opacity { NumberAnimation { duration: 200 } }
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: root.accent
                    strokeWidth: Math.max(3, root.s * 0.018)
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    startX: -root.s * 0.055
                    startY: 0
                    PathQuad {
                        x: root.s * 0.055
                        y: 0
                        controlX: 0
                        controlY: root.sleepy || root.offline ? root.s * 0.02 : root.s * 0.055
                    }
                }
            }
        }

        // ---- Thinking bubbles --------------------------------------------------

        Repeater {
            model: 3
            delegate: Rectangle {
                required property int index
                readonly property real size: root.s * (0.025 + index * 0.012)
                width: size
                height: size
                radius: size / 2
                color: root.accent
                x: face.width * (0.70 + index * 0.07) - size / 2
                y: face.height * (0.22 - index * 0.07) + hop
                opacity: root.mood === "thinking" ? 0.85 : 0
                Behavior on opacity { NumberAnimation { duration: 200 } }
                property real hop: 0
                SequentialAnimation on hop {
                    running: root.mood === "thinking"
                    loops: Animation.Infinite
                    PauseAnimation { duration: index * 160 }
                    NumberAnimation { to: -root.s * 0.025; duration: 300; easing.type: Easing.OutQuad }
                    NumberAnimation { to: 0; duration: 300; easing.type: Easing.InQuad }
                    PauseAnimation { duration: (2 - index) * 160 + 200 }
                }
            }
        }

        // ---- Sleepy z's ----------------------------------------------------------

        Repeater {
            model: 3
            delegate: Text {
                id: zee
                required property int index
                text: "z"
                color: root.accent
                font.pixelSize: Math.round(root.s * (0.05 + index * 0.015))
                font.bold: true
                x: face.width * 0.68 + drift * root.s * 0.12
                y: face.height * 0.30 - drift * root.s * 0.28
                opacity: (root.sleepy || root.offline) && root.mood === "idle" ? (1 - drift) * 0.7 : 0
                // Each z rides the shared cycle a third of the way apart, so they float up one by one.
                readonly property real drift: (root.zPhase + index / 3) % 1
            }
        }
    }

    // ---- Words: what Luna heard, what she is doing, what she is saying -------

    Column {
        anchors.horizontalCenter: parent.horizontalCenter
        y: face.y + face.height * 0.86
        width: Math.min(root.width * 0.88, root.s * 1.25)
        spacing: root.s * 0.03

        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            color: root.accent
            opacity: 0.8
            font.pixelSize: Math.round(root.s * 0.042)
            text: {
                if (root.mood !== "thinking") return ""
                switch (backend.tool) {
                case "web_search": return qsTr("Searching the web…")
                case "web_extract": case "browser": return qsTr("Reading a page…")
                case "terminal": return qsTr("Working on it…")
                case "memory": return qsTr("Remembering…")
                default: return qsTr("Thinking…")
                }
            }
        }

        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            color: "#9aa0aa"
            font.italic: true
            font.pixelSize: Math.round(root.s * 0.045)
            text: backend.heard ? qsTr("“%1”").arg(backend.heard) : ""
            visible: text !== "" && (root.mood === "thinking" || root.mood === "speaking")
        }

        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            color: "#ecebe8"
            font.pixelSize: Math.round(root.s * 0.06)
            lineHeight: 1.1
            text: root.mood === "speaking" ? backend.speech : ""
            visible: text !== ""
        }
    }

    // A quiet hint while she waits, or plainly what is wrong.
    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: root.s * 0.08
        width: root.width * 0.9
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        font.pixelSize: Math.round(root.s * 0.04)
        color: root.offline ? "#e0a44a" : "#6b7280"
        text: root.offline ? qsTr("Luna can't hear right now: her voice service (voiced) isn't running.")
                           : qsTr("Say “Hey Luna”")
        visible: root.mood === "idle"
    }
}
