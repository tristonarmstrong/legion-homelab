import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

ApplicationWindow {
    id: win
    width: 800
    height: 1280
    visible: true
    visibility: snapshotMode ? Window.Windowed : Window.FullScreen
    title: "Luna"
    color: "#07080b"

    Material.theme: Material.Dark
    Material.accent: theme.accent
    Material.roundedScale: theme.radius

    property bool helpVisible: false

    Shortcut { sequence: "?"; context: Qt.ApplicationShortcut; onActivated: win.helpVisible = !win.helpVisible }
    Shortcut { sequence: "Q"; context: Qt.ApplicationShortcut; onActivated: Qt.quit() }
    Shortcut { sequence: "Space"; context: Qt.ApplicationShortcut; onActivated: backend.stepDemo() }
    Shortcut {
        sequence: "F"
        context: Qt.ApplicationShortcut
        onActivated: win.visibility = win.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen
    }
    Shortcut { sequence: "Esc"; context: Qt.ApplicationShortcut; onActivated: win.helpVisible = false }

    // Right-to-left languages mirror the window; the Translator sets the
    // application's layout direction.
    Item {
        anchors.fill: parent
        LayoutMirroring.enabled: Qt.application.layoutDirection === Qt.RightToLeft
        LayoutMirroring.childrenInherit: true

        LunaFace { anchors.fill: parent }

        Text {
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.topMargin: 24
            visible: backend.demo
            color: theme.accent
            font.pixelSize: 18
            text: qsTr("Preview: Space steps through Luna's moods")
        }

        Rectangle {
            anchors.centerIn: parent
            visible: win.helpVisible
            width: Math.min(parent.width * 0.85, 460)
            height: help.implicitHeight + 48
            radius: theme.radius
            color: "#16181d"
            border.color: Qt.rgba(1, 1, 1, 0.08)

            GridLayout {
                id: help
                anchors.centerIn: parent
                columns: 2
                columnSpacing: 28
                rowSpacing: 12
                Repeater {
                    model: [
                        ["Space", qsTr("Preview Luna's moods")],
                        ["F", qsTr("Full screen on or off")],
                        ["?", qsTr("Show or hide shortcuts")],
                        ["Q", qsTr("Quit")]
                    ]
                    delegate: RowLayout {
                        required property var modelData
                        Layout.columnSpan: 2
                        spacing: 28
                        Label {
                            text: modelData[0]
                            color: theme.accent
                            font.bold: true
                            Layout.preferredWidth: 64
                        }
                        Label { text: modelData[1]; color: "#ecebe8" }
                    }
                }
            }
        }
    }
}
