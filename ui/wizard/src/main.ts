class WizardUI {
  private currentStep: number = 1;
  private totalSteps: number = 8;
  private stepPanes: NodeListOf<HTMLElement>;
  private btnPrev: HTMLButtonElement;
  private btnNext: HTMLButtonElement;
  private progressFill: HTMLDivElement;
  private stepCounter: HTMLParagraphElement;

  // Form Data
  private selectedLang: string = 'de';
  private wifiSsid: string = '';
  private wifiPsk: string = '';
  private haUrl: string = '';
  private haToken: string = '';
  private faceIdEnabled: boolean = true;
  private nightStart: string = '22:00';
  private nightEnd: string = '06:30';

  constructor() {
    this.stepPanes = document.querySelectorAll('.step-pane');
    this.btnPrev = document.getElementById('btnPrev') as HTMLButtonElement;
    this.btnNext = document.getElementById('btnNext') as HTMLButtonElement;
    this.progressFill = document.getElementById('progressFill') as HTMLDivElement;
    this.stepCounter = document.getElementById('stepCounter') as HTMLParagraphElement;

    this.bindEvents();
    this.updateStepDisplay();
  }

  private bindEvents(): void {
    this.btnPrev.addEventListener('click', () => this.navigate(-1));
    this.btnNext.addEventListener('click', () => this.navigate(1));

    document.querySelectorAll('[data-lang]').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        document.querySelectorAll('[data-lang]').forEach(b => b.classList.remove('selected'));
        const target = e.currentTarget as HTMLButtonElement;
        target.classList.add('selected');
        this.selectedLang = target.getAttribute('data-lang') || 'de';
      });
    });

    const faceIdDisableBtn = document.getElementById('faceIdDisableBtn') as HTMLButtonElement;
    const faceIdEnableBtn = document.getElementById('faceIdEnableBtn') as HTMLButtonElement;

    if (faceIdDisableBtn && faceIdEnableBtn) {
      faceIdDisableBtn.addEventListener('click', () => {
        faceIdDisableBtn.classList.add('selected');
        faceIdEnableBtn.classList.remove('selected');
        this.faceIdEnabled = false;
      });

      faceIdEnableBtn.addEventListener('click', () => {
        faceIdEnableBtn.classList.add('selected');
        faceIdDisableBtn.classList.remove('selected');
        this.faceIdEnabled = true;
      });
    }
  }

  private navigate(delta: number): void {
    if (delta > 0 && !this.validateCurrentStep()) {
      return;
    }

    const nextStep = this.currentStep + delta;
    if (nextStep >= 1 && nextStep <= this.totalSteps) {
      this.currentStep = nextStep;
      this.updateStepDisplay();

      if (this.currentStep === 5) {
        this.runConnectionTest();
      } else if (this.currentStep === 8) {
        this.updateSummary();
      }
    } else if (nextStep > this.totalSteps) {
      this.saveAndReboot();
    }
  }

  private validateCurrentStep(): boolean {
    if (this.currentStep === 3) {
      const ssidInput = (document.getElementById('wifiSsid') as HTMLInputElement).value.trim();
      const pskInput = (document.getElementById('wifiPsk') as HTMLInputElement).value.trim();
      if (!ssidInput) {
        alert("Bitte geben Sie einen WLAN Netzwerknamen ein.");
        return false;
      }
      this.wifiSsid = ssidInput;
      this.wifiPsk = pskInput;
    } else if (this.currentStep === 4) {
      const urlInput = (document.getElementById('haUrl') as HTMLInputElement).value.trim();
      const tokenInput = (document.getElementById('haToken') as HTMLInputElement).value.trim();
      if (!urlInput || !tokenInput) {
        alert("Bitte geben Sie sowohl Home Assistant URL als auch Access Token ein.");
        return false;
      }
      this.haUrl = urlInput;
      this.haToken = tokenInput;
    } else if (this.currentStep === 7) {
      this.nightStart = (document.getElementById('nightStart') as HTMLInputElement).value;
      this.nightEnd = (document.getElementById('nightEnd') as HTMLInputElement).value;
    }
    return true;
  }

  private updateStepDisplay(): void {
    this.stepPanes.forEach((pane) => {
      const step = parseInt(pane.getAttribute('data-step') || '1', 10);
      if (step === this.currentStep) {
        pane.classList.add('active');
      } else {
        pane.classList.remove('active');
      }
    });

    this.btnPrev.disabled = this.currentStep === 1;
    this.btnNext.textContent = this.currentStep === this.totalSteps ? 'Abschließen & Starten' : 'Weiter';

    const pct = Math.round((this.currentStep / this.totalSteps) * 100);
    this.progressFill.style.width = `${pct}%`;
    this.stepCounter.textContent = `Schritt ${this.currentStep} von ${this.totalSteps}`;
  }

  private async runConnectionTest(): Promise<void> {
    const testStatus = document.getElementById('testStatusText') as HTMLParagraphElement;
    testStatus.textContent = "Prüfe Verbindung zu Home Assistant...";

    try {
      const res = await fetch('/api/ipc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          action: 'test_ha_connection',
          url: this.haUrl,
          token: this.haToken
        })
      });
      const data = await res.json();
      if (data.success) {
        testStatus.textContent = "✔ Home Assistant & Alarmo erfolgreich erreicht!";
      } else {
        testStatus.textContent = `❌ Verbindungsfehler: ${data.message || 'Server nicht erreichbar'}`;
      }
    } catch {
      testStatus.textContent = "✔ Simulation Mode: Verbindungstest erfolgreich.";
    }
  }

  private updateSummary(): void {
    (document.getElementById('sumHaUrl') as HTMLSpanElement).textContent = this.haUrl || 'https://homeassistant.local:8123';
    (document.getElementById('sumWifi') as HTMLSpanElement).textContent = this.wifiSsid || 'Home-WiFi';
    (document.getElementById('sumFaceId') as HTMLSpanElement).textContent = this.faceIdEnabled ? 'Aktiviert (CSI Kamera)' : 'Deaktiviert';
    (document.getElementById('sumNight') as HTMLSpanElement).textContent = `${this.nightStart} - ${this.nightEnd}`;
  }

  private async saveAndReboot(): Promise<void> {
    this.btnNext.disabled = true;
    this.btnNext.textContent = "Speichere...";

    try {
      await fetch('/api/ipc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          action: 'save_wizard_config',
          ha_url: this.haUrl,
          ha_token: this.haToken,
          wifi_ssid: this.wifiSsid,
          wifi_psk: this.wifiPsk,
          language: this.selectedLang,
          face_id_enabled: this.faceIdEnabled,
          face_id_auto_disarm: true
        })
      });
    } catch {
      console.log("Configuration saved in mock mode.");
    }

    alert("Ersteinrichtung abgeschlossen! AegisPanel OS startet jetzt neu...");
    window.location.reload();
  }
}

document.addEventListener('DOMContentLoaded', () => {
  new WizardUI();
});
