#include "power_mgmt.h"
#include "esp_timer.h"
#include "esp_log.h"

static const char* TAG = "POWER_MGMT_MCU";

PowerManagerMCU::PowerManagerMCU(gpio_num_t wake_pin)
    : m_wake_pin(wake_pin), m_wake_active(false), m_last_presence_time_ms(0) {}

void PowerManagerMCU::init() {
    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << m_wake_pin),
        .mode = GPIO_MODE_OUTPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_ENABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_config(&io_conf);
    setWakeSignal(false);
    ESP_LOGI(TAG, "GPIO Wake Pin %d initialized as Output", m_wake_pin);
}

void PowerManagerMCU::setWakeSignal(bool active) {
    if (m_wake_active != active) {
        m_wake_active = active;
        gpio_set_level(m_wake_pin, active ? 1 : 0);
        ESP_LOGI(TAG, "Wake Signal -> %s", active ? "HIGH (WAKE)" : "LOW (IDLE)");
    }
}

void PowerManagerMCU::updatePresenceState(bool presence_in_range, uint32_t hold_time_ms) {
    uint64_t now_ms = esp_timer_get_time() / 1000;

    if (presence_in_range) {
        m_last_presence_time_ms = now_ms;
        setWakeSignal(true);
    } else {
        if (m_wake_active && (now_ms - m_last_presence_time_ms > hold_time_ms)) {
            setWakeSignal(false);
        }
    }
}
