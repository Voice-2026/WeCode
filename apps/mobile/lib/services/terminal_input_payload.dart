String terminalPastePayload(String text) {
  if (text.length <= 1) {
    return text;
  }
  return '\u001b[200~$text\u001b[201~';
}
