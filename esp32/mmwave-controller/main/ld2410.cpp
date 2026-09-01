#include "ld2410.h"
#include "esp_log.h"
#include <cstring>
#include <algorithm>

static const char* TAG = "LD2410";

LD2410Sensor::LD2410Sensor(uart_port_t uart_num, int tx_pin, int rx_pin)
    : m_uart_num(uart_num), m_tx_pin(tx_pin), m_rx_pin(rx_pin) {}

bool LD2410Sensor::init(uint32_t baud_rate) {
    uart_config_t uart_config = {
        .baud_rate = (int)baud_rate,
        .data_bits = UART_DATA_8_BITS,
        .parity    = UART_PARITY_DISABLE,
        .stop_bits = UART_STOP_BITS_1,
        .flow_ctrl = UART_HW_FLOWCTRL_DISABLE,
        .rx_flow_ctrl_thresh = 122,
        .source_clk = UART_SCLK_APB,
    };

    if (uart_param_config(m_uart_num, &uart_config) != ESP_OK) return false;
    if (uart_set_pin(m_uart_num, m_tx_pin, m_rx_pin, UART_PIN_NO_CHANGE, UART_PIN_NO_CHANGE) != ESP_OK) return false;
    if (uart_driver_install(m_uart_num, 1024, 0, 0, NULL, 0) != ESP_OK) return false;

    ESP_LOGI(TAG, "HLK-LD2410 UART initialized on TX:%d RX:%d at %lu baud", m_tx_pin, m_rx_pin, baud_rate);
    return true;
}

bool LD2410Sensor::readFrame(RadarTargetInfo& info) {
    info.presence = false;
    info.moving_target = false;
    info.stationary_target = false;
    info.moving_distance_cm = 9999;
    info.stationary_distance_cm = 9999;
    info.min_distance_cm = 9999;

    int length = 0;
    uart_get_buffered_data_len(m_uart_num, (size_t*)&length);
    if (length < 23) return false;

    int rx_bytes = uart_read_bytes(m_uart_num, m_buffer, sizeof(m_buffer) - 1, pdMS_TO_TICKS(50));
    if (rx_bytes <= 0) return false;

    for (int i = 0; i < rx_bytes - 22; i++) {
        // Frame header check: 0xF4 0xF3 0xF2 0xF1
        if (m_buffer[i] == 0xF4 && m_buffer[i+1] == 0xF3 && m_buffer[i+2] == 0xF2 && m_buffer[i+3] == 0xF1) {
            uint8_t target_state = m_buffer[i + 8];
            
            info.presence = (target_state != 0x00);
            info.moving_target = (target_state == 0x01 || target_state == 0x03);
            info.stationary_target = (target_state == 0x02 || target_state == 0x03);

            info.moving_distance_cm = m_buffer[i + 9] | (m_buffer[i + 10] << 8);
            info.stationary_distance_cm = m_buffer[i + 12] | (m_buffer[i + 13] << 8);

            if (info.moving_target && info.stationary_target) {
                info.min_distance_cm = std::min(info.moving_distance_cm, info.stationary_distance_cm);
            } else if (info.moving_target) {
                info.min_distance_cm = info.moving_distance_cm;
            } else if (info.stationary_target) {
                info.min_distance_cm = info.stationary_distance_cm;
            }

            return true;
        }
    }

    return false;
}
