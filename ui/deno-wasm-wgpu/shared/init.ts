import Process from "node:process";

export function setProcessTitle(title: string) {
  Process.title = title;
}
