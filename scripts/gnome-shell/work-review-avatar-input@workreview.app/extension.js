import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Clutter from 'gi://Clutter';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const DBUS_PATH = '/org/gnome/shell/extensions/WorkReviewAvatarInput';
const DBUS_XML = `
<node>
  <interface name="org.gnome.shell.extensions.WorkReviewAvatarInput">
    <method name="GetPointer">
      <arg type="s" name="payload" direction="out"/>
    </method>
  </interface>
</node>`;

function mouseGroupFromModifiers(modifiers) {
  if (modifiers & Clutter.ModifierType.BUTTON1_MASK)
    return 'mouse-left';

  if (modifiers & Clutter.ModifierType.BUTTON3_MASK)
    return 'mouse-right';

  if (modifiers & Clutter.ModifierType.BUTTON2_MASK)
    return 'mouse-side';

  return 'mouse-move';
}

class WorkReviewAvatarInputService {
  GetPointer() {
    const [x, y, modifiers] = global.get_pointer();

    return JSON.stringify({
      x: Math.round(x),
      y: Math.round(y),
      mouseGroup: mouseGroupFromModifiers(modifiers),
      timestampMs: Math.floor(GLib.get_monotonic_time() / 1000),
    });
  }
}

export default class WorkReviewAvatarInputExtension extends Extension {
  enable() {
    if (this._dbusObject)
      return;

    this._service = new WorkReviewAvatarInputService();
    this._dbusObject = Gio.DBusExportedObject.wrapJSObject(DBUS_XML, this._service);
    this._dbusObject.export(Gio.DBus.session, DBUS_PATH);
  }

  disable() {
    this._dbusObject?.unexport();
    this._dbusObject = null;
    this._service = null;
  }
}
