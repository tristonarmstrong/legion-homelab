#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTemporaryDir>
#include <QtTest>

#include "backend.h"
#include "theme.h"
#include "translator.h"

// Builds one voiced message. (Raw string literals here would hide the class from moc.)
static QString msg(const QJsonObject &object)
{
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}

class LunaTests : public QObject {
    Q_OBJECT

private slots:
    void followsVoicedStates()
    {
        Backend backend(QUrl(), false);
        QCOMPARE(backend.state(), QString("idle"));
        backend.handleMessage(msg({{"state", "wake"}, {"score", 0.9}}));
        QCOMPARE(backend.state(), QString("wake"));
        backend.handleMessage(msg({{"state", "listening"}}));
        QCOMPARE(backend.state(), QString("listening"));
    }

    void heardTextKeepsHerThinking()
    {
        Backend backend(QUrl(), false);
        backend.handleMessage(msg({{"state", "thinking"}}));
        backend.handleMessage(msg({{"state", "heard"}, {"text", "Pull up a recipe."}}));
        QCOMPARE(backend.state(), QString("thinking"));
        QCOMPARE(backend.heard(), QString("Pull up a recipe."));
    }

    void toolOnlyWhileThinking()
    {
        Backend backend(QUrl(), false);
        backend.handleMessage(msg({{"state", "thinking"}, {"tool", "web_search"}}));
        QCOMPARE(backend.tool(), QString("web_search"));
        backend.handleMessage(msg({{"state", "speaking"}, {"text", "It's up."}}));
        QCOMPARE(backend.tool(), QString());
        QCOMPARE(backend.speech(), QString("It's up."));
    }

    void newTurnClearsOldWords()
    {
        Backend backend(QUrl(), false);
        backend.handleMessage(msg({{"state", "heard"}, {"text", "Hi"}}));
        backend.handleMessage(msg({{"state", "speaking"}, {"text", "Hello!"}}));
        backend.handleMessage(msg({{"state", "listening"}}));
        QCOMPARE(backend.speech(), QString());
        backend.handleMessage(msg({{"state", "idle"}}));
        QCOMPARE(backend.heard(), QString());
    }

    void ignoresJunk()
    {
        Backend backend(QUrl(), false);
        backend.handleMessage("not json");
        backend.handleMessage(msg({{"text", "no state"}}));
        QCOMPARE(backend.state(), QString("idle"));
    }

    void demoStepsAndVoicedEndsIt()
    {
        Backend backend(QUrl(), false);
        backend.stepDemo();
        QVERIFY(backend.demo());
        QCOMPARE(backend.state(), QString("wake"));
        backend.handleMessage(msg({{"state", "idle"}}));
        QVERIFY(!backend.demo());
    }

    void themeFallsBack()
    {
        QTemporaryDir dir;
        Theme theme(dir.path());
        QCOMPARE(theme.accent(), QString("#FFD60A"));
        QCOMPARE(theme.accentForeground(), QString("black"));
    }

    void themeReadsAccent()
    {
        QTemporaryDir dir;
        QDir(dir.path()).mkpath("theme");
        write(dir.path() + "/theme/colors.toml", "accent = \"#112233\"\n");
        Theme theme(dir.path());
        QCOMPARE(theme.accent(), QString("#112233"));
        QCOMPARE(theme.accentForeground(), QString("white"));
    }

    void translatorFallsBackToEnglish()
    {
        QTemporaryDir dir;
        Translator translator(dir.path() + "/language", dir.path());
        QCOMPARE(translator.language(), QString("en"));
        QVERIFY(translator.translate("Main", "Thinking…").isNull());
    }

private:
    static void write(const QString &path, const QByteArray &contents)
    {
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write(contents);
    }
};

QTEST_MAIN(LunaTests)
#include "luna_tests.moc"
