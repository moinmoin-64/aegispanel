class SecurityUI {
  private pin: string = "";
  private pinDots: NodeListOf<HTMLSpanElement>;
  private errorBanner: HTMLDivElement;
  private keypadWrapper: HTMLDivElement;
  private isSubmitting: boolean = false;

  constructor() {
    this.pinDots = document.querySelectorAll('.dot');
    this.errorBanner = document.getElementById('errorBanner') as HTMLDivElement;
    this.keypadWrapper = document.querySelector('.keypad-wrapper') as HTMLDivElement;

    this.bindEvents();
  }

  private bindEvents(): void {
    // Touch & Mouse events for on-screen keypad
    const buttons = document.querySelectorAll<HTMLButtonElement>('.key-btn');
    buttons.forEach((btn) => {
      btn.addEventListener('click', () => {
        const key = btn.getAttribute('data-key');
        if (key) {
          this.handleKeyPress(key);
        }
      });
    });

    // Physical / USB keyboard support
    window.addEventListener('keydown', (e: KeyboardEvent) => {
      if (this.isSubmitting) return;

      if (e.key >= '0' && e.key <= '9') {
        this.handleKeyPress(e.key);
      } else if (e.key === 'Backspace') {
        this.handleBackSpace();
      } else if (e.key === 'Escape' || e.key === 'Delete') {
        this.handleKeyPress('clear');
      } else if (e.key === 'Enter') {
        this.handleKeyPress('enter');
      }
    });
  }

  private handleBackSpace(): void {
    if (this.pin.length > 0) {
      this.pin = this.pin.slice(0, -1);
      this.updateDots();
      this.hideError();
    }
  }

  private handleKeyPress(key: string): void {
    if (this.isSubmitting) return;

    this.hideError();

    if (key === 'clear') {
      this.clearPin();
    } else if (key === 'enter') {
      this.submitPin();
    } else if (this.pin.length < 8 && /^[0-9]$/.test(key)) {
      this.pin += key;
      this.updateDots();
    }
  }

  private updateDots(): void {
    this.pinDots.forEach((dot, index) => {
      if (index < this.pin.length) {
        dot.classList.add('active');
      } else {
        dot.classList.remove('active');
      }
    });
  }

  private clearPin(): void {
    this.pin = "";
    this.updateDots();
  }

  private showError(msg: string): void {
    if (this.errorBanner) {
      this.errorBanner.textContent = msg;
      this.errorBanner.classList.add('visible');
    }
    if (this.keypadWrapper) {
      this.keypadWrapper.classList.add('shake');
      setTimeout(() => {
        this.keypadWrapper.classList.remove('shake');
      }, 450);
    }
  }

  private hideError(): void {
    if (this.errorBanner) {
      this.errorBanner.classList.remove('visible');
    }
  }

  private async submitPin(): void {
    if (this.pin.length < 4) {
      this.showError('PIN zu kurz (min. 4 Stellen)');
      return;
    }

    this.isSubmitting = true;
    const pinToSend = this.pin;
    
    // Immediate memory clear in JS context
    this.clearPin();

    try {
      // IPC post to AegisPanel Core bridge
      const response = await fetch('/api/ipc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'disarm', pin: pinToSend }),
      });

      const data = await response.json();

      if (data.success) {
        // Success handled by Core daemon switching display to Kiosk
      } else {
        this.showError(data.message || 'Falscher PIN');
      }
    } catch {
      // Fallback mock check for local standalone testing
      console.warn("IPC endpoint unreachable. Simulating backend disarm request check...");
      this.showError('PIN Falsch (Alarmo)');
    } finally {
      this.isSubmitting = false;
    }
  }
}

document.addEventListener('DOMContentLoaded', () => {
  new SecurityUI();
});
