#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "ld2410.h"
#include "power_mgmt.h"

static const char* TAG = "AEGIS_MMWAVE_MAIN";

#define UART_NUM       UART_NUM_1
#define UART_TX_PIN    (21)
#define UART_RX_PIN    (20)
#define WAKE_GPIO_PIN  (GPIO_NUM_4)
#define TARGET_DISTANCE_THRESHOLD_CM (100) // 1 Meter Envelope

extern "C" void app_main(void) {
    ESP_LOGI(TAG, "===============================================");
    ESP_LOGI(TAG, " AegisPanel OS - ESP32-C3 mmWave Controller    ");
    ESP_LOGI(TAG, "===============================================");

    LD2410Sensor radar(UART_NUM, UART_TX_PIN, UART_RX_PIN);
    PowerManagerMCU power_mcu(WAKE_GPIO_PIN);

    power_mcu.init();

    if (!radar.init(256000)) {
        ESP_LOGE(TAG, "Failed to initialize HLK-LD2410 radar sensor!");
        return;
    }

    RadarTargetInfo info;

    while (1) {
        if (radar.readFrame(info)) {
            bool in_range = info.presence && (info.min_distance_cm <= TARGET_DISTANCE_THRESHOLD_CM);
            
            if (in_range) {
                ESP_LOGI(TAG, "Target DETECTED in range (%u cm) -> WAKE", info.min_distance_cm);
            }

            power_mcu.updatePresenceState(in_range, 5000);
        }

        vTaskDelay(pdMS_TO_TICKS(100)); // 100ms loop delay
    }
}
