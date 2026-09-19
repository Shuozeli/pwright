// One API suite catalog for both legacy golden lanes and Node FFI comparison.
import pageGoto from "../specs/page-goto.spec.js";
import pageEvaluate from "../specs/page-evaluate.spec.js";
import pageContent from "../specs/page-content.spec.js";
import locatorQuery from "../specs/locator-query.spec.js";
import locatorClick from "../specs/locator-click.spec.js";
import locatorFill from "../specs/locator-fill.spec.js";
import keyboard from "../specs/keyboard.spec.js";
import mouse from "../specs/mouse.spec.js";
import screenshot from "../specs/screenshot.spec.js";
import download from "../specs/download.spec.js";
import fileUpload from "../specs/file-upload.spec.js";
import locatorText from "../specs/locator-text.spec.js";
import locatorLabel from "../specs/locator-label.spec.js";
import locatorRole from "../specs/locator-role.spec.js";
import locatorFilter from "../specs/locator-filter.spec.js";
import locatorCompose from "../specs/locator-compose.spec.js";
import touchscreen from "../specs/touchscreen.spec.js";
import navigationExtras from "../specs/navigation-extras.spec.js";
import actionExtras from "../specs/action-extras.spec.js";
import type { Suite } from "./harness.js";

export const API_SUITES: readonly Suite[] = [
  pageGoto, pageEvaluate, pageContent, locatorQuery, locatorClick, locatorFill,
  keyboard, mouse, screenshot, download, fileUpload, locatorText, locatorLabel,
  locatorRole, locatorFilter, locatorCompose, touchscreen, navigationExtras, actionExtras,
];
