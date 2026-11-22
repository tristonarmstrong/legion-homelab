QT += core gui quick quickcontrols2 testlib websockets
CONFIG += c++17 testcase
TARGET = luna_tests
TEMPLATE = app

INCLUDEPATH += ../src

HEADERS += ../src/backend.h ../src/theme.h ../src/translator.h
SOURCES += luna_tests.cpp ../src/backend.cpp ../src/theme.cpp ../src/translator.cpp
