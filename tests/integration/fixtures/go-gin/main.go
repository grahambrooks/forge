package main

import "github.com/gin-gonic/gin"

func main() {
	r := gin.Default()
	r.GET("/items", listItems)
	r.POST("/items", createItem)
	r.GET("/health", healthCheck)
	r.Run(":8080")
}

func listItems(c *gin.Context)   { c.JSON(200, []string{}) }
func createItem(c *gin.Context)  { c.JSON(201, gin.H{}) }
func healthCheck(c *gin.Context) { c.String(200, "ok") }
