package main

import "github.com/gin-gonic/gin"

func main() {
	r := gin.Default()
	r.GET("/orders", listOrders)
	r.POST("/orders", createOrder)
	r.Run(":8080")
}

func listOrders(c *gin.Context)  {}
func createOrder(c *gin.Context) {}
