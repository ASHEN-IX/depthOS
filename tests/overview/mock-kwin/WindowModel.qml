import QtQuick
ListModel {
    dynamicRoles: true
    property int revision: 0
    function touch() { revision++; }
}
