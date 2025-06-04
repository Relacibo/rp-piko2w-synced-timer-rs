# pico2w-synced-alarmtimer

> **Hinweis:** Viel Code in diesem Projekt und dieses README wurden von GitHub Copilot generiert und anschließend manuell angepasst.

## Verwendung

1. Klone das Repository:
   ```
   git clone <repository-url>
   ```

2. Navigiere in das Projektverzeichnis:
   ```
   cd rp-piko2w-synced-timer-rs
   ```

3. Baue das Projekt:
   ```
   just build
   ```

4. Flashe das Projekt auf den Raspberry Pi Pico 2W (z.B. mit probe-rs oder elf2uf2 + Drag&Drop).

5. **WLAN-Konfiguration:**  
   Beim ersten Start öffnet das Gerät einen WLAN-Access-Point (SoftAP) mit dem Namen `Pico2W-Setup`.  
   Verbinde dich mit diesem WLAN und öffne im Browser die Seite [http://192.168.4.1](http://192.168.4.1), um deine WLAN-Zugangsdaten einzugeben.  
   Die Daten werden im Flash gespeichert.

6. **Zurücksetzen der Zugangsdaten:**  
   Halte beim Starten die Setup-Taste (z.B. an GPIO15) für 5 Sekunden gedrückt, um die gespeicherten WLAN-Zugangsdaten zu löschen und den Setup-Modus erneut zu aktivieren.

## Lizenz

Dieses Projekt steht unter der MIT-Lizenz. Weitere Informationen findest du in der LICENSE-Datei.
